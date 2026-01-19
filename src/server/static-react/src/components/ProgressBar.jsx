import React from 'react'

/**
 * Progress Bar Component
 * Displays ingestion progress with step-by-step information
 */
function ProgressBar({ progress, className = '' }) {
  if (!progress) {
    return null
  }

  const getStepColor = (step) => {
    switch (step) {
      case 'ValidatingConfig':
        return 'bg-blue-500'
      case 'PreparingSchemas':
        return 'bg-indigo-500'
      case 'FlatteningData':
        return 'bg-purple-500'
      case 'GettingAIRecommendation':
        return 'bg-pink-500'
      case 'SettingUpSchema':
        return 'bg-red-500'
      case 'GeneratingMutations':
        return 'bg-orange-500'
      case 'ExecutingMutations':
        return 'bg-yellow-500'
      case 'Completed':
        return 'bg-green-500'
      case 'Failed':
        return 'bg-red-600'
      default:
        return 'bg-gray-500'
    }
  }

  const getStepLabel = (step) => {
    switch (step) {
      case 'ValidatingConfig':
        return 'Validating Configuration'
      case 'PreparingSchemas':
        return 'Preparing Schemas'
      case 'FlatteningData':
        return 'Processing Data'
      case 'GettingAIRecommendation':
        return 'AI Analysis'
      case 'SettingUpSchema':
        return 'Setting Up Schema'
      case 'GeneratingMutations':
        return 'Generating Mutations'
      case 'ExecutingMutations':
        return 'Executing Mutations'
      case 'Completed':
        return 'Completed'
      case 'Failed':
        return 'Failed'
      default:
        return step
    }
  }

  const formatDuration = (startedAt, completedAt) => {
    const start = new Date(startedAt * 1000)
    const end = completedAt ? new Date(completedAt * 1000) : new Date()
    const duration = Math.round((end - start) / 1000)
    
    if (duration < 60) {
      return `${duration}s`
    } else {
      const minutes = Math.floor(duration / 60)
      const seconds = duration % 60
      return `${minutes}m ${seconds}s`
    }
  }

  return (
    <div className={`bg-white p-4 rounded-lg shadow border ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <div className={`w-3 h-3 rounded-full ${getStepColor(progress.current_step)}`}></div>
          <h3 className="text-sm font-medium text-gray-900">
            {getStepLabel(progress.current_step)}
          </h3>
        </div>
        <div className="text-xs text-gray-500">
          {formatDuration(progress.started_at, progress.completed_at)}
        </div>
      </div>

      {/* Progress Bar */}
      <div className="mb-3">
        <div className="flex justify-between text-xs text-gray-600 mb-1">
          <span>{progress.progress_percentage}%</span>
          <span>{progress.status_message}</span>
        </div>
        <div className="w-full bg-gray-200 rounded-full h-2">
          <div
            className={`h-2 rounded-full transition-all duration-300 ${getStepColor(progress.current_step)}`}
            style={{ width: `${progress.progress_percentage}%` }}
          ></div>
        </div>
      </div>

      {/* Results */}
      {progress.results && (
        <div className="mt-3 p-3 bg-green-50 rounded-md">
          <div className="text-sm text-green-800">
            <div className="font-medium mb-1">Ingestion Complete!</div>
            <div className="text-xs space-y-1">
              <div>Schema: {progress.results.schema_name}</div>
              <div>New Schema: {progress.results.new_schema_created ? 'Yes' : 'No'}</div>
              <div>Mutations Generated: {progress.results.mutations_generated}</div>
              <div>Mutations Executed: {progress.results.mutations_executed}</div>
            </div>
          </div>
        </div>
      )}

      {/* Error */}
      {progress.error_message && (
        <div className="mt-3 p-3 bg-red-50 rounded-md">
          <div className="text-sm text-red-800">
            <div className="font-medium mb-1">Ingestion Failed</div>
            <div className="text-xs">{progress.error_message}</div>
          </div>
        </div>
      )}

      {/* Step Indicators */}
      <div className="mt-4">
        <div className="flex justify-between text-xs text-gray-500">
          {[
            'ValidatingConfig',
            'PreparingSchemas', 
            'FlatteningData',
            'GettingAIRecommendation',
            'SettingUpSchema',
            'GeneratingMutations',
            'ExecutingMutations'
          ].map((step, index) => {
            const isActive = progress.current_step === step
            const isCompleted = progress.progress_percentage > (index + 1) * 12.5
            
            return (
              <div key={step} className="flex flex-col items-center">
                <div
                  className={`w-2 h-2 rounded-full mb-1 ${
                    isActive || isCompleted
                      ? getStepColor(step)
                      : 'bg-gray-300'
                  }`}
                ></div>
                <span className="text-xs text-center max-w-16 leading-tight">
                  {getStepLabel(step).split(' ')[0]}
                </span>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}

export default ProgressBar
