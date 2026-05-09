-- PDF ETL 数据库初始化脚本
-- 创建扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";  -- 用于模糊搜索
CREATE EXTENSION IF NOT EXISTS "btree_gin"; -- 用于复合索引

-- 创建提取记录表
CREATE TABLE IF NOT EXISTS extracted_documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    schema_name VARCHAR(255) NOT NULL,
    data JSONB NOT NULL,
    source_file VARCHAR(512),
    extraction_metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_extracted_docs_schema_name ON extracted_documents(schema_name);
CREATE INDEX IF NOT EXISTS idx_extracted_docs_created_at ON extracted_documents(created_at);
CREATE INDEX IF NOT EXISTS idx_extracted_docs_data_gin ON extracted_documents USING GIN(data);
CREATE INDEX IF NOT EXISTS idx_extracted_docs_source_file ON extracted_documents(source_file);
CREATE INDEX IF NOT EXISTS idx_extracted_docs_schema_trgm ON extracted_documents USING GIN(schema_name gin_trgm_ops);

-- 创建处理任务表
CREATE TABLE IF NOT EXISTS processing_tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    file_path VARCHAR(512) NOT NULL,
    schema_name VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',  -- pending, processing, completed, failed
    processing_path VARCHAR(50),  -- fast_path, vision_path, hybrid_path
    result_id UUID REFERENCES extracted_documents(id),
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    metadata JSONB
);

-- 创建任务索引
CREATE INDEX IF NOT EXISTS idx_tasks_status ON processing_tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_schema_name ON processing_tasks(schema_name);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON processing_tasks(created_at);

-- 创建验证记录表
CREATE TABLE IF NOT EXISTS validation_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID REFERENCES extracted_documents(id) ON DELETE CASCADE,
    is_valid BOOLEAN NOT NULL,
    score DECIMAL(5,4) NOT NULL,
    errors JSONB,
    warnings JSONB,
    needs_review BOOLEAN DEFAULT FALSE,
    validated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建验证索引
CREATE INDEX IF NOT EXISTS idx_validation_document_id ON validation_records(document_id);
CREATE INDEX IF NOT EXISTS idx_validation_needs_review ON validation_records(needs_review) WHERE needs_review = TRUE;

-- 创建审计日志表
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100),
    resource_id VARCHAR(255),
    user_id VARCHAR(255),
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建审计索引
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_created_at ON audit_logs(created_at);

-- 创建 Schema 定义表
CREATE TABLE IF NOT EXISTS schema_definitions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL UNIQUE,
    schema JSONB NOT NULL,
    rust_type VARCHAR(255),
    description TEXT,
    version INTEGER DEFAULT 1,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建 Schema 索引
CREATE INDEX IF NOT EXISTS idx_schema_name ON schema_definitions(name);
CREATE INDEX IF NOT EXISTS idx_schema_active ON schema_definitions(is_active) WHERE is_active = TRUE;

-- 创建更新时间触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 为 extracted_documents 创建触发器
CREATE TRIGGER update_extracted_documents_updated_at
    BEFORE UPDATE ON extracted_documents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- 为 schema_definitions 创建触发器
CREATE TRIGGER update_schema_definitions_updated_at
    BEFORE UPDATE ON schema_definitions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- 创建统计视图
CREATE OR REPLACE VIEW extraction_statistics AS
SELECT 
    schema_name,
    COUNT(*) as total_documents,
    COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '24 hours') as documents_last_24h,
    COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '7 days') as documents_last_7d,
    AVG(
        (extraction_metadata->>'processing_time_ms')::INTEGER
    ) FILTER (WHERE extraction_metadata->>'processing_time_ms' IS NOT NULL) as avg_processing_time_ms
FROM extracted_documents
GROUP BY schema_name;

-- 创建任务统计视图
CREATE OR REPLACE VIEW task_statistics AS
SELECT 
    schema_name,
    COUNT(*) as total_tasks,
    COUNT(*) FILTER (WHERE status = 'pending') as pending_tasks,
    COUNT(*) FILTER (WHERE status = 'processing') as processing_tasks,
    COUNT(*) FILTER (WHERE status = 'completed') as completed_tasks,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_tasks,
    AVG(
        EXTRACT(EPOCH FROM (completed_at - started_at))
    ) FILTER (WHERE status = 'completed' AND started_at IS NOT NULL AND completed_at IS NOT NULL) as avg_duration_seconds
FROM processing_tasks
GROUP BY schema_name;

-- 插入默认 Schema 定义
INSERT INTO schema_definitions (name, schema, description) VALUES
('InvoiceSchema', 
 '{
   "type": "object",
   "properties": {
     "invoice_number": {"type": "string", "description": "发票编号"},
     "date": {"type": "string", "description": "发票日期"},
     "vendor": {
       "type": "object",
       "properties": {
         "name": {"type": "string"},
         "address": {"type": "string"},
         "tax_id": {"type": "string"}
       }
     },
     "items": {
       "type": "array",
       "items": {
         "type": "object",
         "properties": {
           "description": {"type": "string"},
           "quantity": {"type": "integer"},
           "unit_price": {"type": "number"},
           "amount": {"type": "number"}
         }
       }
     },
     "total_amount": {"type": "number"},
     "currency": {"type": "string"}
   }
 }'::jsonb,
 '发票提取 Schema'),
 
('ContractSchema',
 '{
   "type": "object",
   "properties": {
     "contract_number": {"type": "string", "description": "合同编号"},
     "title": {"type": "string", "description": "合同标题"},
     "party_a": {
       "type": "object",
       "properties": {
         "name": {"type": "string"},
         "address": {"type": "string"},
         "contact": {"type": "string"}
       }
     },
     "party_b": {
       "type": "object",
       "properties": {
         "name": {"type": "string"},
         "address": {"type": "string"},
         "contact": {"type": "string"}
       }
     },
     "sign_date": {"type": "string"},
     "effective_date": {"type": "string"},
     "expiry_date": {"type": "string"},
     "amount": {"type": "number"},
     "terms": {"type": "array", "items": {"type": "string"}}
   }
 }'::jsonb,
 '合同提取 Schema'),

('ResumeSchema',
 '{
   "type": "object",
   "properties": {
     "name": {"type": "string"},
     "email": {"type": "string"},
     "phone": {"type": "string"},
     "work_experience": {
       "type": "array",
       "items": {
         "type": "object",
         "properties": {
           "company": {"type": "string"},
           "position": {"type": "string"},
           "start_date": {"type": "string"},
           "end_date": {"type": "string"},
           "description": {"type": "string"}
         }
       }
     },
     "education": {
       "type": "array",
       "items": {
         "type": "object",
         "properties": {
           "school": {"type": "string"},
           "degree": {"type": "string"},
           "major": {"type": "string"},
           "graduation_year": {"type": "integer"}
         }
       }
     },
     "skills": {"type": "array", "items": {"type": "string"}}
   }
 }'::jsonb,
 '简历提取 Schema')
ON CONFLICT (name) DO NOTHING;

-- 授权 (根据实际用户调整)
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO pdf_user;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO pdf_user;
