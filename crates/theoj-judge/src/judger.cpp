// SPDX-License-Identifier: AGPL-3.0-only
//
// https://github.com/lrw04/jury
// Copyright (C) 2023 lrw04 <2428592483@qq.com>
//
// https://github.com/paraN3xus/theoj
// Copyright (C) 2025 ParaN3xus <paran3xus007@gmail.com>

#include <cerrno>
#include <cstdio>
#include <cstring>
#include <fcntl.h>
#include <fstream>
#include <iostream>
#include <sched.h>
#include <signal.h>
#include <sstream>
#include <string>
#include <sys/mount.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <vector>

const int EXTRA_TIME = 1000;
const int STACK_SIZE = 1024 * 1024;

enum Verdict {
  VERDICT_OK = 0,
  VERDICT_TLE = 1,
  VERDICT_MLE = 2,
  VERDICT_RE = 3,
  VERDICT_UKE = 4
};

struct FileInfo {
  std::string filename;
  std::vector<char> content;
  int mode;
};

struct JudgeConfig {
  int time_limit;         // ms
  long long memory_limit; // MB
  int pids_limit;
  std::string rootfs;
  std::string tmpfs_size;
  std::string cgroup;
  std::string sandbox_id;
  std::string stdin_content;
  std::vector<std::string> cmdline;
  std::vector<FileInfo> input_files;
  std::vector<std::string> output_filenames;
};

struct JudgeResult {
  int verdict;
  int time;         // ms
  long long memory; // KB -> MB later
  std::string stdout_content;
  std::string stderr_content;
  std::vector<FileInfo> output_files;
};

// utils
void write_full(int fd, const void *buf, size_t size) {
  size_t total = 0;
  while (total < size) {
    ssize_t ret = write(fd, (const char *)buf + total, size - total);
    if (ret < 0) {
      if (errno == EINTR)
        continue;
      throw std::runtime_error("Write failed");
    }
    total += ret;
  }
}

void read_full(int fd, void *buf, size_t size) {
  size_t total = 0;
  while (total < size) {
    ssize_t ret = read(fd, (char *)buf + total, size - total);
    if (ret <= 0) {
      if (errno == EINTR)
        continue;
      throw std::runtime_error("Read failed or EOF");
    }
    total += ret;
  }
}

// proto
std::string read_proto_str(int fd) {
  int len;
  read_full(fd, &len, sizeof(int));
  if (len == 0)
    return "";
  std::string s(len, '\0');
  read_full(fd, &s[0], len);
  return s;
}

void write_proto_str(int fd, const std::string &s) {
  int len = s.length();
  write_full(fd, &len, sizeof(int));
  if (len > 0)
    write_full(fd, s.data(), len);
}

void write_proto_buf(int fd, const std::vector<char> &buf) {
  int len = buf.size();
  write_full(fd, &len, sizeof(int));
  if (len > 0)
    write_full(fd, buf.data(), len);
}

// file utils

void write_file(const std::string &path, const std::string &content) {
  std::ofstream ofs(path);
  if (!ofs)
    throw std::runtime_error("Failed to write text: " + path);
  ofs << content;
}

void write_bin_file(const std::string &path, const std::vector<char> &content,
                    int mode) {
  int fd = open(path.c_str(), O_WRONLY | O_CREAT | O_TRUNC, mode);
  if (fd < 0)
    throw std::runtime_error("Failed to create file: " + path);
  size_t written = 0;
  while (written < content.size()) {
    ssize_t ret = write(fd, content.data() + written, content.size() - written);
    if (ret < 0) {
      close(fd);
      throw std::runtime_error("Write file error");
    }
    written += ret;
  }
  close(fd);
}

std::string read_file(const std::string &path) {
  std::ifstream ifs(path);
  if (!ifs)
    return "";
  std::stringstream ss;
  ss << ifs.rdbuf();
  return ss.str();
}

std::vector<char> read_bin_file(const std::string &path) {
  std::ifstream ifs(path, std::ios::binary | std::ios::ate);
  if (!ifs)
    return {};
  std::streamsize size = ifs.tellg();
  ifs.seekg(0, std::ios::beg);
  std::vector<char> buf(size);
  if (ifs.read(buf.data(), size))
    return buf;
  return {};
}

std::string get_cgroup_key(const std::string &s, const std::string &k) {
  std::stringstream ss(s);
  std::string key, val;
  while (ss >> key >> val) {
    if (key == k)
      return val;
  }
  return "0";
}

// sandbox
struct RunContext {
  JudgeConfig *cfg;
  int child_pipe[2]; // barrier
  int result_pipe[2];
};

int sandbox_executor(RunContext *ctx) {
  close(ctx->result_pipe[0]);
  close(ctx->result_pipe[1]);

  if (chdir(("/tmp/judger_sandbox_" + ctx->cfg->sandbox_id + "/tmp").c_str()))
    return 1;

  // redir stdio
  write_file("stdin", ctx->cfg->stdin_content);
  setuid(65534); // nobody
  setgid(65534);
  freopen("stdin", "r", stdin);
  freopen("stdout", "w", stdout);
  freopen("stderr", "w", stderr);

  // wait cgroup proc
  close(ctx->child_pipe[1]);
  char ch;
  if (read(ctx->child_pipe[0], &ch, 1) <= 0)
    return 1;
  close(ctx->child_pipe[0]);

  // block SIGCHLD
  sigset_t mask;
  sigemptyset(&mask);
  sigaddset(&mask, SIGCHLD);
  if (sigprocmask(SIG_BLOCK, &mask, nullptr) == -1) {
    return 1; // sys err
  }

  int pid = fork();
  if (pid < 0)
    return 1;

  if (pid == 0) {
    // SIG_UNBLOCK
    sigprocmask(SIG_UNBLOCK, &mask, nullptr);

    char *envp[] = {(char *)"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/"
                            "usr/bin:/sbin:/bin",
                    nullptr};
    rlimit rl;
    rl.rlim_cur = rl.rlim_max = RLIM_INFINITY;
    setrlimit(RLIMIT_STACK, &rl);

    std::vector<char *> argv;
    for (auto &s : ctx->cfg->cmdline)
      argv.push_back(&s[0]);
    argv.push_back(nullptr);

    execve(argv[0], argv.data(), envp);
    exit(EXIT_FAILURE);
  }

  int time_limit_ms = ctx->cfg->time_limit + EXTRA_TIME;
  struct timespec timeout;
  timeout.tv_sec = time_limit_ms / 1000;
  timeout.tv_nsec = (time_limit_ms % 1000) * 1000000;

  int sig_ret = sigtimedwait(&mask, nullptr, &timeout);

  if (sig_ret < 0) {
    if (errno == EAGAIN) {
      // tle
      kill(pid, SIGKILL);
      // prevent zombie
      waitpid(pid, nullptr, 0);
      return 2; // TLE
    } else {
      // other
      kill(pid, SIGKILL);
      waitpid(pid, nullptr, 0);
      return 1;
    }
  }

  // exited
  int status;
  int wait_ret = waitpid(pid, &status, WNOHANG);

  if (wait_ret == pid) {
    // normal exit check
    return (WIFEXITED(status) ? (WEXITSTATUS(status) == 0 ? 0 : 1) : 3);
  }

  // default
  kill(pid, SIGKILL);
  return 1;
}

int container_init(void *arg) {
  RunContext *ctx = (RunContext *)arg;

  // wait set uid
  close(ctx->child_pipe[1]);
  char ch;
  if (read(ctx->child_pipe[0], &ch, 1) <= 0)
    return 1;

  // set namespace
  if (sethostname("sandbox", 7) ||
      mount(nullptr, "/", nullptr, MS_REC | MS_PRIVATE, nullptr))
    return 1;

  std::string sandbox_root = "/tmp/judger_sandbox_" + ctx->cfg->sandbox_id;
  mkdir(sandbox_root.c_str(), 0777);

  // mount overlay/bind
  if (mount(ctx->cfg->rootfs.c_str(), sandbox_root.c_str(), "", MS_BIND, "") ||
      mount("", sandbox_root.c_str(), "", MS_REMOUNT | MS_RDONLY | MS_BIND, ""))
    return 1;

  std::string tmp_path = sandbox_root + "/tmp";
  std::string opts = "mode=0777,size=" + ctx->cfg->tmpfs_size;
  if (mount("tmpfs", tmp_path.c_str(), "tmpfs", 0, opts.c_str()))
    return 1;

  // write files
  for (const auto &f : ctx->cfg->input_files) {
    try {
      write_bin_file(tmp_path + "/" + f.filename, f.content, f.mode);
    } catch (...) {
      return 1;
    }
  }

  // cgroup
  std::string cgroup_path = ctx->cfg->cgroup + "/judge." + ctx->cfg->sandbox_id;
  mkdir(cgroup_path.c_str(), 0755);

  close(ctx->child_pipe[0]);
  if (pipe2(ctx->child_pipe, O_CLOEXEC) == -1)
    return 1;

  // restrict resource
  try {
    write_file(cgroup_path + "/cpu.max", "100000 100000");
    write_file(cgroup_path + "/pids.max", std::to_string(ctx->cfg->pids_limit));
    std::string mem_limit =
        std::to_string(ctx->cfg->memory_limit * 1024 * 1024);
    write_file(cgroup_path + "/memory.max", mem_limit);
    write_file(cgroup_path + "/memory.swap.max", "0");
  } catch (...) {
    return 1;
  }

  char *stack = new char[STACK_SIZE];
  int exec_pid = clone(
      (int (*)(void *))sandbox_executor, stack + STACK_SIZE,
      CLONE_NEWNS | CLONE_NEWNET | CLONE_NEWPID | CLONE_NEWUTS | SIGCHLD, ctx);

  if (exec_pid < 0)
    return 1;

  // add executor to the cgroup
  write_file(cgroup_path + "/cgroup.procs", std::to_string(exec_pid));

  write(ctx->child_pipe[1], "1", 1);
  close(ctx->child_pipe[1]);

  int status;
  waitpid(exec_pid, &status, 0);
  delete[] stack;

  // collect res
  JudgeResult res;
  res.verdict = VERDICT_UKE;

  int exit_code = 255;
  if (WIFEXITED(status))
    exit_code = WEXITSTATUS(status);

  std::string cpu_stat = read_file(cgroup_path + "/cpu.stat");
  std::string mem_peak = read_file(cgroup_path + "/memory.peak");
  std::string mem_events = read_file(cgroup_path + "/memory.events");

  res.time = stoll(get_cgroup_key(cpu_stat, "user_usec")) / 1000;
  res.memory = (mem_peak.empty() ? 0 : stoll(mem_peak)) / 1024 / 1024;
  int oom = stoi(get_cgroup_key(mem_events, "oom_kill"));

  if (exit_code == 0)
    res.verdict = VERDICT_OK;
  else if (exit_code == 1)
    res.verdict = VERDICT_RE;
  else if (exit_code == 2)
    res.verdict = VERDICT_TLE;
  else
    res.verdict = VERDICT_UKE;

  if (oom)
    res.verdict = VERDICT_MLE;
  if (res.time > ctx->cfg->time_limit)
    res.verdict = VERDICT_TLE;

  res.stdout_content = read_file(tmp_path + "/stdout");
  res.stderr_content = read_file(tmp_path + "/stderr");

  for (const auto &fname : ctx->cfg->output_filenames) {
    res.output_files.push_back(
        {fname, read_bin_file(tmp_path + "/" + fname), 0});
  }

  // clean
  rmdir(cgroup_path.c_str());
  umount(tmp_path.c_str());
  umount(sandbox_root.c_str());
  rmdir(sandbox_root.c_str());

  // write results
  write_full(ctx->result_pipe[1], &res.verdict, sizeof(int));
  write_full(ctx->result_pipe[1], &res.time, sizeof(int));
  write_full(ctx->result_pipe[1], &res.memory, sizeof(long long));
  write_proto_str(ctx->result_pipe[1], res.stdout_content);
  write_proto_str(ctx->result_pipe[1], res.stderr_content);

  int file_cnt = res.output_files.size();
  write_full(ctx->result_pipe[1], &file_cnt, sizeof(int));
  for (auto &f : res.output_files) {
    write_proto_str(ctx->result_pipe[1], f.filename);
    write_proto_buf(ctx->result_pipe[1], f.content);
  }

  close(ctx->result_pipe[1]);
  return 0;
}

int main() {
  signal(SIGPIPE, SIG_IGN);

  try {
    JudgeConfig cfg;

    read_full(0, &cfg.time_limit, sizeof(int));
    read_full(0, &cfg.memory_limit, sizeof(long long));
    read_full(0, &cfg.pids_limit, sizeof(int));
    cfg.rootfs = read_proto_str(0);
    cfg.tmpfs_size = read_proto_str(0);
    cfg.cgroup = read_proto_str(0);
    cfg.sandbox_id = read_proto_str(0);
    cfg.stdin_content = read_proto_str(0);

    int count;
    read_full(0, &count, sizeof(int)); // cmdline
    for (int i = 0; i < count; ++i)
      cfg.cmdline.push_back(read_proto_str(0));

    read_full(0, &count, sizeof(int)); // input files
    for (int i = 0; i < count; ++i) {
      FileInfo fi;
      fi.filename = read_proto_str(0);
      int sz;
      read_full(0, &sz, sizeof(int));
      fi.content.resize(sz);
      if (sz > 0)
        read_full(0, fi.content.data(), sz);
      read_full(0, &fi.mode, sizeof(int));
      cfg.input_files.push_back(fi);
    }

    read_full(0, &count, sizeof(int)); // output filenames
    for (int i = 0; i < count; ++i)
      cfg.output_filenames.push_back(read_proto_str(0));

    // prepare ctx
    RunContext ctx;
    ctx.cfg = &cfg;
    if (pipe2(ctx.child_pipe, O_CLOEXEC) < 0)
      throw std::runtime_error("pipe");
    if (pipe2(ctx.result_pipe, O_CLOEXEC) < 0)
      throw std::runtime_error("pipe");

    // launch namespace container
    char *stack = new char[STACK_SIZE];
    int ns_pid = clone(container_init, stack + STACK_SIZE,
                       CLONE_NEWUSER | CLONE_NEWNS | CLONE_NEWIPC |
                           CLONE_NEWNET | CLONE_NEWUTS | SIGCHLD,
                       &ctx);
    if (ns_pid < 0)
      throw std::runtime_error("clone failed");

    // uid map
    try {
      std::string uid_map = "/proc/" + std::to_string(ns_pid) + "/uid_map";
      std::string gid_map = "/proc/" + std::to_string(ns_pid) + "/gid_map";
      write_file("/proc/" + std::to_string(ns_pid) + "/setgroups", "deny");
      write_file(uid_map, "0 " + std::to_string(getuid()) + " 1");
      write_file(gid_map, "0 " + std::to_string(getgid()) + " 1");
    } catch (...) {
      kill(ns_pid, SIGKILL);
      throw;
    }

    write(ctx.child_pipe[1], "1", 1);
    close(ctx.child_pipe[0]);
    close(ctx.child_pipe[1]);

    // wait
    int status;
    waitpid(ns_pid, &status, 0);
    delete[] stack;

    close(ctx.result_pipe[1]);

    int verdict, time, file_cnt;
    long long memory;

    read_full(ctx.result_pipe[0], &verdict, sizeof(int));
    read_full(ctx.result_pipe[0], &time, sizeof(int));
    read_full(ctx.result_pipe[0], &memory, sizeof(long long));
    std::string stdout_str = read_proto_str(ctx.result_pipe[0]);
    std::string stderr_str = read_proto_str(ctx.result_pipe[0]);

    read_full(ctx.result_pipe[0], &file_cnt, sizeof(int));
    std::vector<FileInfo> out_files;
    for (int i = 0; i < file_cnt; ++i) {
      FileInfo f;
      f.filename = read_proto_str(ctx.result_pipe[0]);
      int sz;
      read_full(ctx.result_pipe[0], &sz, sizeof(int));
      f.content.resize(sz);
      if (sz > 0)
        read_full(ctx.result_pipe[0], f.content.data(), sz);
      out_files.push_back(f);
    }

    // output
    write_full(1, &verdict, sizeof(int));
    write_full(1, &time, sizeof(int));
    write_full(1, &memory, sizeof(long long));
    write_proto_str(1, stdout_str);
    write_proto_str(1, stderr_str);
    write_full(1, &file_cnt, sizeof(int));
    for (auto &f : out_files) {
      write_proto_str(1, f.filename);
      write_proto_buf(1, f.content);
    }

  } catch (const std::exception &e) {
    // UKE
    int v = VERDICT_UKE, t = 0;
    long long m = 0;
    std::string msg = "Internal Error: ";
    msg += e.what();

    write_full(1, &v, sizeof(int));
    write_full(1, &t, sizeof(int));
    write_full(1, &m, sizeof(long long));
    write_proto_str(1, "");
    write_proto_str(1, msg); // Stderr
    int zero = 0;
    write_full(1, &zero, sizeof(int)); // 0 files
    return 1;
  }

  return 0;
}