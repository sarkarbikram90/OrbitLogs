-- Create replay_sessions table for historical replay metadata
CREATE TABLE IF NOT EXISTS replay_sessions (
    session_id UUID PRIMARY KEY,
    satellite_id VARCHAR(50),
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time TIMESTAMP WITH TIME ZONE NOT NULL,
    current_position TIMESTAMP WITH TIME ZONE NOT NULL,
    speed REAL NOT NULL DEFAULT 1.0,
    status VARCHAR(20) NOT NULL CHECK (status IN ('PLAYING', 'PAUSED', 'STOPPED')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
