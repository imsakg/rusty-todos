-- TODO Status enum
CREATE TYPE todo_status_enum AS ENUM (
    'open',
    'close'
);

-- TODO
CREATE TABLE todo (
    id bigserial,
    cid bigint NOT NULL, -- creator userID
    ctime timestamp with time zone DEFAULT now(),
    mid bigint,
    mtime timestamp with time zone,
    title text NOT NULL,
    status todo_status_enum NOT NULL DEFAULT 'open'
); 
ALTER SEQUENCE todo_id_seq RESTART WITH 1000;

