import { useState, useEffect } from 'react'
import { getDatabaseConfig, updateDatabaseConfig, resetDatabase } from '../../api/clients/systemClient'
import { ingestionClient } from '../../api/clients'
import { TrashIcon } from '@heroicons/react/24/solid'

function DatabaseSettings({ configSaveStatus, setConfigSaveStatus, onClose }) {
  const [dbType, setDbType] = useState('local')
  const [dbPath, setDbPath] = useState('data')
  const [dynamoTableName, setDynamoTableName] = useState('DataFoldStorage')
  const [dynamoRegion, setDynamoRegion] = useState('us-west-2')
  const [dynamoUserId, setDynamoUserId] = useState('')
  const [s3Bucket, setS3Bucket] = useState('')
  const [s3Region, setS3Region] = useState('us-east-1')
  const [s3Prefix, setS3Prefix] = useState('folddb')
  const [s3LocalPath, setS3LocalPath] = useState('/tmp/folddb-data')
  const [isResetting, setIsResetting] = useState(false)
  const [resetResult, setResetResult] = useState(null)

  useEffect(() => {
    loadDatabaseConfig()
  }, [])

  const loadDatabaseConfig = async () => {
    try {
      const response = await getDatabaseConfig()
      if (response.success && response.data) {
        const config = response.data
        setDbType(config.type)
        if (config.type === 'local') {
          setDbPath(config.path || 'data')
        } else if (config.type === 'dynamodb') {
          setDynamoTableName(config.table_name || 'DataFoldStorage')
          setDynamoRegion(config.region || 'us-west-2')
          setDynamoUserId(config.user_id || '')
        } else if (config.type === 's3') {
          setS3Bucket(config.bucket || '')
          setS3Region(config.region || 'us-east-1')
          setS3Prefix(config.prefix || 'folddb')
          setS3LocalPath(config.local_path || '/tmp/folddb-data')
        }
      }
    } catch (error) {
      console.error('Failed to load database config:', error)
    }
  }

  const saveDatabaseConfig = async () => {
    try {
      let config
      if (dbType === 'local') {
        config = {
          type: 'local',
          path: dbPath
        }
      } else if (dbType === 'dynamodb') {
        if (!dynamoTableName || !dynamoRegion) {
          setConfigSaveStatus({ success: false, message: 'Table name and region are required for DynamoDB' })
          setTimeout(() => setConfigSaveStatus(null), 3000)
          return
        }
        config = {
          type: 'dynamodb',
          table_name: dynamoTableName,
          region: dynamoRegion,
          user_id: dynamoUserId || undefined
        }
      } else if (dbType === 's3') {
        if (!s3Bucket || !s3Region) {
          setConfigSaveStatus({ success: false, message: 'Bucket and region are required for S3' })
          setTimeout(() => setConfigSaveStatus(null), 3000)
          return
        }
        config = {
          type: 's3',
          bucket: s3Bucket,
          region: s3Region,
          prefix: s3Prefix || 'folddb',
          local_path: s3LocalPath || '/tmp/folddb-data'
        }
      }

      const response = await updateDatabaseConfig(config)
      
      if (response.success) {
        setConfigSaveStatus({ 
          success: true, 
          message: response.data.requires_restart 
            ? 'Database configuration saved. Please restart the server for changes to take effect.'
            : response.data.message || 'Database configuration saved and restarted successfully'
        })
        setTimeout(() => {
          setConfigSaveStatus(null)
          if (!response.data.requires_restart) {
            onClose()
          }
        }, 3000)
      } else {
        setConfigSaveStatus({ success: false, message: response.error || 'Failed to save database configuration' })
      }
    } catch (error) {
      setConfigSaveStatus({ success: false, message: error.message || 'Failed to save database configuration' })
    }
    setTimeout(() => setConfigSaveStatus(null), 5000)
  }

  const handleResetDatabase = async () => {
    setIsResetting(true)
    setResetResult(null)

    try {
      const response = await resetDatabase(true)

      if (response.success && response.data) {
        if (response.data.job_id) {
          const jobId = response.data.job_id
          
          const pollInterval = setInterval(async () => {
            try {
              const progressResponse = await ingestionClient.getJobProgress(jobId)
              if (progressResponse.success && progressResponse.data) {
                const progressData = progressResponse.data
                if (progressData.is_complete) {
                  clearInterval(pollInterval)
                  setResetResult({ type: 'success', message: 'Database reset complete. Reloading...' })
                  setTimeout(() => {
                    window.location.reload()
                  }, 1000)
                } else if (progressData.is_failed) {
                  clearInterval(pollInterval)
                  setResetResult({ type: 'error', message: progressData.error_message || 'Reset failed' })
                  setIsResetting(false)
                }
              }
            } catch {
              // Continue polling on network errors
            }
          }, 1000)
          
          setTimeout(() => {
            clearInterval(pollInterval)
            if (isResetting) {
              setResetResult({ type: 'success', message: 'Reset likely complete. Reloading...' })
              setTimeout(() => {
                window.location.reload()
              }, 1000)
            }
          }, 60000)
        } else {
          setResetResult({ type: 'success', message: response.data.message || 'Database reset successfully' })
          setTimeout(() => {
            window.location.reload()
          }, 2000)
        }
      } else {
        setResetResult({ type: 'error', message: response.error || 'Reset failed' })
        setIsResetting(false)
      }
    } catch (error) {
      setResetResult({ type: 'error', message: `Network error: ${error.message}` })
      setIsResetting(false)
    }
  }

  return {
    saveDatabaseConfig,
    content: (
      <div className="space-y-4">
        <div className="mb-4">
          <h4 className="text-md font-semibold text-success mb-2"># Database Storage Backend</h4>
          <p className="text-sm text-secondary mb-4">
            Choose the storage backend for your database. Changes require a server restart.
          </p>
        </div>

        <div>
          <label className="block text-sm font-medium text-secondary mb-2">
            --storage-type
          </label>
          <select
            value={dbType}
            onChange={(e) => setDbType(e.target.value)}
            className="w-full p-2 border border-gray-200 bg-white text-primary text-sm"
          >
            <option value="local">Local (Sled)</option>
            <option value="dynamodb">DynamoDB</option>
            <option value="s3">S3</option>
          </select>
        </div>

        {dbType === 'local' ? (
          <div>
            <label className="block text-sm font-medium text-secondary mb-1">
              --path
            </label>
            <input
              type="text"
              value={dbPath}
              onChange={(e) => setDbPath(e.target.value)}
              placeholder="data"
              className="w-full p-2 border border-gray-200 bg-white text-primary text-sm"
            />
            <p className="text-xs text-secondary mt-1">
              Local filesystem path where the database will be stored
            </p>
          </div>
        ) : dbType === 'dynamodb' ? (
          <div className="space-y-3">
            <div>
              <label className="block text-sm font-medium text-secondary mb-1">
                Table Name <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                value={dynamoTableName}
                onChange={(e) => setDynamoTableName(e.target.value)}
                placeholder="DataFoldStorage"
                className="w-full p-2 border border-gray-200 text-sm"
              />
              <p className="text-xs text-secondary mt-1">
                Base table name (namespaces will be appended automatically)
              </p>
            </div>
            <div>
              <label className="block text-sm font-medium text-secondary mb-1">
                AWS Region <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                value={dynamoRegion}
                onChange={(e) => setDynamoRegion(e.target.value)}
                placeholder="us-west-2"
                className="w-full p-2 border border-gray-200 text-sm"
              />
              <p className="text-xs text-secondary mt-1">
                AWS region where your DynamoDB tables are located
              </p>
            </div>
            <div>
              <label className="block text-sm font-medium text-secondary mb-1">
                User ID (Optional)
              </label>
              <input
                type="text"
                value={dynamoUserId}
                onChange={(e) => setDynamoUserId(e.target.value)}
                placeholder="Leave empty for single-tenant"
                className="w-full p-2 border border-gray-200 text-sm"
              />
              <p className="text-xs text-secondary mt-1">
                User ID for multi-tenant isolation (uses partition key)
              </p>
            </div>
            <div className="p-3 minimal-card border border-yellow-400 ">
              <p className="text-xs text-warning">
                <strong>Note:</strong> Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). 
                The DynamoDB tables will be created automatically if they don't exist.
              </p>
            </div>
          </div>
        ) : (
          <div className="space-y-3">
            <div>
              <label className="block text-sm font-medium text-secondary mb-1">
                S3 Bucket <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                value={s3Bucket}
                onChange={(e) => setS3Bucket(e.target.value)}
                placeholder="my-datafold-bucket"
                className="w-full p-2 border border-gray-200 text-sm"
              />
              <p className="text-xs text-secondary mt-1">
                S3 bucket name where the database will be stored
              </p>
            </div>
            <div>
              <label className="block text-sm font-medium text-secondary mb-1">
                AWS Region <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                value={s3Region}
                onChange={(e) => setS3Region(e.target.value)}
                placeholder="us-east-1"
                className="w-full p-2 border border-gray-200 text-sm"
              />
              <p className="text-xs text-secondary mt-1">
                AWS region where your S3 bucket is located
              </p>
            </div>
            <div>
              <label className="block text-sm font-medium text-secondary mb-1">
                S3 Prefix (Optional)
              </label>
              <input
                type="text"
                value={s3Prefix}
                onChange={(e) => setS3Prefix(e.target.value)}
                placeholder="folddb"
                className="w-full p-2 border border-gray-200 text-sm"
              />
              <p className="text-xs text-secondary mt-1">
                Prefix/path within the bucket (defaults to "folddb")
              </p>
            </div>
            <div>
              <label className="block text-sm font-medium text-secondary mb-1">
                Local Cache Path
              </label>
              <input
                type="text"
                value={s3LocalPath}
                onChange={(e) => setS3LocalPath(e.target.value)}
                placeholder="/tmp/folddb-data"
                className="w-full p-2 border border-gray-200 text-sm"
              />
              <p className="text-xs text-secondary mt-1">
                Local filesystem path for caching S3 data (defaults to /tmp/folddb-data)
              </p>
            </div>
            <div className="p-3 minimal-card border border-yellow-400 ">
              <p className="text-xs text-warning">
                <strong>Note:</strong> Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). 
                The database will be synced to/from S3 on startup and shutdown.
              </p>
            </div>
          </div>
        )}

        {/* Danger Zone - Reset Database */}
        <div className="mt-8 pt-6 border-t border-red-500">
          <div className="flex items-center gap-2 mb-3">
            <TrashIcon className="w-5 h-5 text-red-500" />
            <h4 className="text-md font-semibold text-error">Danger Zone</h4>
          </div>
          <p className="text-sm text-secondary mb-4">
            Permanently delete all data and restart the database. This action cannot be undone.
          </p>
          
          {!isResetting ? (
            <button
              onClick={handleResetDatabase}
              className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-red-600 border border-red-300 hover:bg-red-50 hover:border-red-400 transition-colors"
            >
              <TrashIcon className="w-4 h-4" />
              Reset Database
            </button>
          ) : (
            <div className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-info border border-blue-300">
              <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              Resetting Database...
            </div>
          )}

          {resetResult && (
            <div className={`mt-4 p-3 text-sm ${
              resetResult.type === 'success'
                ? 'text-success border border-green-500'
                : resetResult.type === 'info'
                  ? 'text-info border border-blue-300'
                  : 'minimal-card text-error border border-red-500'
              }`}>
              {resetResult.type === 'info' && (
                <svg className="inline-block animate-spin h-4 w-4 mr-2" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                </svg>
              )}
              {resetResult.message}
            </div>
          )}
        </div>

        {configSaveStatus && (
          <div className={`p-3  ${
            configSaveStatus.success 
              ? 'text-success border border-green-500' 
              : 'minimal-card text-error border border-red-500'
          }`}>
            <span className="text-sm font-medium">
              {configSaveStatus.success ? '✓' : '✗'} {configSaveStatus.message}
            </span>
          </div>
        )}
      </div>
    )
  }
}

export default DatabaseSettings
