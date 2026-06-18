-- Create mission plans table for scheduling activities
CREATE TABLE IF NOT EXISTS mission_planning (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    plan_type VARCHAR(50) NOT NULL, -- 'PAYLOAD', 'IMAGE', 'MANEUVER', 'UPDATE', 'COMMAND'
    description TEXT NOT NULL,
    scheduled_time TIMESTAMP WITH TIME ZONE NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('DRAFT', 'REVIEW', 'APPROVED', 'SCHEDULED', 'EXECUTED', 'CANCELLED')),
    created_by VARCHAR(50) NOT NULL,
    approved_by VARCHAR(50),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create ssa conjunction events history table
CREATE TABLE IF NOT EXISTS ssa_conjunctions (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    object_name VARCHAR(100) NOT NULL,
    object_type VARCHAR(50) NOT NULL, -- 'DEBRIS', 'SATELLITE'
    probability DOUBLE PRECISION NOT NULL, -- 0.0 to 1.0
    distance_meters DOUBLE PRECISION NOT NULL,
    conjunction_time TIMESTAMP WITH TIME ZONE NOT NULL,
    severity VARCHAR(20) NOT NULL, -- 'INFO', 'WARNING', 'CRITICAL'
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create operator training scorecard table
CREATE TABLE IF NOT EXISTS operator_training_scores (
    id UUID PRIMARY KEY,
    operator_name VARCHAR(50) NOT NULL,
    scenario_name VARCHAR(100) NOT NULL,
    response_time_seconds DOUBLE PRECISION NOT NULL,
    score INT NOT NULL,
    actions_taken JSONB NOT NULL DEFAULT '[]'::jsonb,
    passed BOOLEAN NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create knowledge graph edges table for impact analysis
CREATE TABLE IF NOT EXISTS knowledge_graph_edges (
    source_type VARCHAR(50) NOT NULL, -- 'SATELLITE', 'GROUND_STATION', 'COMMAND', 'ALERT', 'INCIDENT', 'OPERATOR', 'MISSION', 'ANOMALY'
    source_id VARCHAR(100) NOT NULL,
    target_type VARCHAR(50) NOT NULL,
    target_id VARCHAR(100) NOT NULL,
    relation VARCHAR(50) NOT NULL, -- 'UPLINKED_BY', 'AFFECTS', 'PRECEDES', 'CAUSE_OF', 'MEMBER_OF'
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (source_type, source_id, target_type, target_id, relation)
);
