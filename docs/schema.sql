CREATE TYPE user_role_enum AS ENUM ('admin', 'teacher', 'student', 'guest');
CREATE TYPE user_status_enum AS ENUM ('active', 'inactive');
CREATE TYPE problem_status_enum AS ENUM ('active', 'hidden');
CREATE TYPE contest_type_enum AS ENUM ('public', 'private');
CREATE TYPE contest_status_enum AS ENUM ('active', 'inactive');
CREATE TYPE submission_result_enum AS ENUM (
    'pending', 'accepted', 'wrong_answer', 'time_limit_exceeded', 
    'memory_limit_exceeded', 'runtime_error', 'compile_error', 'unknown_error'
);
CREATE TYPE test_case_result_enum AS ENUM (
    'pending', 'compiling', 'running', 'accepted', 'wrong_answer', 
    'time_limit_exceeded', 'memory_limit_exceeded', 'runtime_error', 
    'compile_error', 'unknown_error'
);

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    phone VARCHAR(32) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(50) UNIQUE NOT NULL,
    user_code VARCHAR(50) UNIQUE NOT NULL,
    user_role user_role_enum NOT NULL,
    password VARCHAR(255) NOT NULL,
    status user_status_enum NOT NULL DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE problems (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    time_limit INTEGER NOT NULL,
    mem_limit INTEGER NOT NULL,
    status problem_status_enum NOT NULL DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE test_cases (
    id SERIAL PRIMARY KEY,
    problem_id INTEGER NOT NULL REFERENCES problems(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE solutions (
    id SERIAL PRIMARY KEY,
    problem_id INTEGER NOT NULL REFERENCES problems(id) ON DELETE CASCADE,
    author INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE contests (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    creator_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    begin_time TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time TIMESTAMP WITH TIME ZONE NOT NULL,
    password VARCHAR(255),
    type contest_type_enum NOT NULL,
    status contest_status_enum NOT NULL DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE submissions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    contest_id INTEGER REFERENCES contests(id) ON DELETE CASCADE,
    problem_id INTEGER NOT NULL REFERENCES problems(id) ON DELETE CASCADE,
    lang VARCHAR(20) NOT NULL,
    result submission_result_enum NOT NULL DEFAULT 'pending',
    time_consumption INTEGER,
    mem_consumption INTEGER,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE submission_test_cases (
    submission_id INTEGER NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,
    test_case_id INTEGER NOT NULL REFERENCES test_cases(id) ON DELETE CASCADE,
    result test_case_result_enum NOT NULL DEFAULT 'pending',
    time_consumption INTEGER,
    mem_consumption INTEGER,
    PRIMARY KEY (submission_id, test_case_id)
);

CREATE TABLE contest_problems (
    contest_id INTEGER NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
    problem_id INTEGER NOT NULL REFERENCES problems(id) ON DELETE CASCADE,
    number INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (contest_id, problem_id)
);

CREATE TABLE training_plans (
    id SERIAL PRIMARY KEY,
    creator_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE training_plan_participants (
    plan_id INTEGER NOT NULL REFERENCES training_plans(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (plan_id, user_id)
);

CREATE TABLE training_plan_contests (
    plan_id INTEGER NOT NULL REFERENCES training_plans(id) ON DELETE CASCADE,
    contest_id INTEGER NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (plan_id, contest_id)
);

CREATE TABLE contest_participants (
    contest_id INTEGER NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    training_plan_id INTEGER NOT NULL DEFAULT 0 REFERENCES training_plans(id) ON DELETE CASCADE,
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (contest_id, user_id, training_plan_id)
);