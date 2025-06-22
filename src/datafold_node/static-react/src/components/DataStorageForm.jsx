import React, { useState, useEffect } from 'react';
import { PaperAirplaneIcon, ExclamationTriangleIcon, ShieldCheckIcon, ArrowPathIcon } from '@heroicons/react/24/outline';
import { useSigning } from '../hooks/useSigning';
import { getAllSchemasWithState, getSchemasByState, approveSchema } from '../api/schemaClient';

const DataStorageForm = ({ keyPair, publicKeyBase64 }) => {
  const [value1, setValue1] = useState('sample-value-1');
  const [value2, setValue2] = useState('sample-value-2');
  const [mutationResult, setMutationResult] = useState(null);
  const [mutationError, setMutationError] = useState(null);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedSchema, setSelectedSchema] = useState('TransformBase');
  const [schemas, setSchemas] = useState({});
  const [schemasLoading, setSchemasLoading] = useState(false);
  const [schemasError, setSchemasError] = useState(null);
  const { signPayload } = useSigning();

  // Fetch schemas with their states on component mount
  useEffect(() => {
    fetchSchemas();
  }, []);

  const fetchSchemas = async () => {
    setSchemasLoading(true);
    setSchemasError(null);
    
    try {
      const response = await getAllSchemasWithState();
      if (response.success) {
        setSchemas(response.data || {});
      } else {
        setSchemasError(response.error || 'Failed to fetch schemas');
      }
    } catch (error) {
      setSchemasError('An error occurred while fetching schemas');
    } finally {
      setSchemasLoading(false);
    }
  };

  const handleApproveSchema = async (schemaName) => {
    try {
      const response = await approveSchema(schemaName);
      if (response.success) {
        // Refresh schemas after approval
        await fetchSchemas();
      } else {
        setSchemasError(response.error || 'Failed to approve schema');
      }
    } catch (error) {
      setSchemasError('An error occurred while approving schema');
    }
  };

  const getSchemaDisplayInfo = (schemaName, state) => {
    const stateColors = {
      available: 'bg-yellow-100 text-yellow-800 border-yellow-200',
      approved: 'bg-green-100 text-green-800 border-green-200',
      blocked: 'bg-red-100 text-red-800 border-red-200'
    };

    const stateLabels = {
      available: 'Unloaded',
      approved: 'Approved',
      blocked: 'Blocked'
    };

    return {
      color: stateColors[state] || 'bg-gray-100 text-gray-800 border-gray-200',
      label: stateLabels[state] || state
    };
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    setMutationResult(null);
    setMutationError(null);
    setIsLoading(true);

    if (!keyPair || !publicKeyBase64) {
      setMutationError("Keypair not available. Please generate and register a key first.");
      setIsLoading(false);
      return;
    }

    const mutationPayload = {
      type: 'mutation',
      schema: selectedSchema,
      mutation_type: 'create',
      data: {
        value1: value1,
        value2: value2,
      },
    };
    
    try {
        const signedMessage = await signPayload(
          mutationPayload,
          publicKeyBase64,
          keyPair.privateKey
        );

        if (!signedMessage) {
            throw new Error('Failed to sign message - please check your key pair');
        }

        const response = await fetch('/api/mutation', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(signedMessage),
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.error || `HTTP error! status: ${response.status}`);
        }

        setMutationResult(data);

    } catch (err) {
        setMutationError(err.message);
    } finally {
        setIsLoading(false);
    }
  };

  return (
    <div className="max-w-4xl mx-auto p-6">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-xl font-semibold text-gray-900">Secure Data Mutation: Create Transform</h2>
          <button
            onClick={fetchSchemas}
            disabled={schemasLoading}
            className="inline-flex items-center px-3 py-1 border border-gray-300 rounded-md text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 disabled:opacity-50"
          >
            <ArrowPathIcon className={`h-4 w-4 mr-1 ${schemasLoading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>
        <p className="text-sm text-gray-600 mb-6">
          This form demonstrates sending a signed data mutation to the backend. The transform creation will be packaged into a mutation, signed on the client-side with your private key, and sent to the server for verification and processing.
        </p>

        {/* Schema Selection Section */}
        <div className="mb-6 p-4 bg-gray-50 rounded-lg">
          <h3 className="text-lg font-medium text-gray-900 mb-3">Available Schemas</h3>
          
          {schemasError && (
            <div className="mb-4 bg-red-50 border border-red-200 rounded-md p-3">
              <div className="flex">
                <ExclamationTriangleIcon className="h-5 w-5 text-red-400 mr-2 flex-shrink-0" />
                <div className="text-sm text-red-700">
                  <p className="font-medium">Schema Loading Error</p>
                  <p>{schemasError}</p>
                </div>
              </div>
            </div>
          )}

          {schemasLoading ? (
            <div className="text-center py-4">
              <div className="inline-flex items-center">
                <ArrowPathIcon className="h-5 w-5 animate-spin mr-2" />
                <span className="text-sm text-gray-600">Loading schemas...</span>
              </div>
            </div>
          ) : (
            <div className="space-y-2">
              {Object.keys(schemas).length === 0 ? (
                <p className="text-sm text-gray-500">No schemas found</p>
              ) : (
                Object.entries(schemas).map(([schemaName, state]) => {
                  const { color, label } = getSchemaDisplayInfo(schemaName, state);
                  const isSelected = selectedSchema === schemaName;
                  const canUse = state === 'approved';
                  
                  return (
                    <div
                      key={schemaName}
                      className={`flex items-center justify-between p-3 border rounded-md ${
                        isSelected ? 'border-blue-500 bg-blue-50' : 'border-gray-200'
                      }`}
                    >
                      <div className="flex items-center space-x-3">
                        <input
                          type="radio"
                          id={`schema-${schemaName}`}
                          name="schema"
                          value={schemaName}
                          checked={isSelected}
                          onChange={(e) => setSelectedSchema(e.target.value)}
                          disabled={!canUse}
                          className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 disabled:opacity-50"
                        />
                        <label
                          htmlFor={`schema-${schemaName}`}
                          className={`text-sm font-medium ${canUse ? 'text-gray-900' : 'text-gray-500'}`}
                        >
                          {schemaName}
                        </label>
                        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${color}`}>
                          {label}
                        </span>
                      </div>
                      
                      {state === 'available' && (
                        <button
                          type="button"
                          onClick={() => handleApproveSchema(schemaName)}
                          className="text-xs bg-blue-600 hover:bg-blue-700 text-white px-2 py-1 rounded"
                        >
                          Approve
                        </button>
                      )}
                    </div>
                  );
                })
              )}
            </div>
          )}
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="selectedSchema" className="block text-sm font-medium text-gray-700">Selected Schema</label>
            <input
              type="text"
              id="selectedSchema"
              value={selectedSchema}
              readOnly
              className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm bg-gray-50 text-gray-700"
            />
          </div>
          <div>
            <label htmlFor="value1" className="block text-sm font-medium text-gray-700">Value 1</label>
            <input
              type="text"
              id="value1"
              value={value1}
              onChange={(e) => setValue1(e.target.value)}
              className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          <div>
            <label htmlFor="value2" className="block text-sm font-medium text-gray-700">Value 2</label>
            <input
              type="text"
              id="value2"
              value={value2}
              onChange={(e) => setValue2(e.target.value)}
              className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          <div>
            <button
              type="submit"
              disabled={isLoading || !keyPair || schemas[selectedSchema] !== 'approved'}
              className="w-full inline-flex items-center justify-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
            >
              <PaperAirplaneIcon className="h-5 w-5 mr-2" />
              {isLoading ? 'Sending...' : 'Sign and Submit Transform Data'}
            </button>
            {schemas[selectedSchema] !== 'approved' && selectedSchema && (
              <p className="text-sm text-yellow-600 mt-2">
                Schema "{selectedSchema}" must be approved before it can be used for mutations.
              </p>
            )}
          </div>
        </form>

        {mutationResult && (
           <div className="mt-6 bg-green-50 border border-green-200 rounded-md p-4">
             <div className="flex">
               <ShieldCheckIcon className="h-5 w-5 text-green-400 mr-2 flex-shrink-0" />
               <div className="text-sm text-green-700">
                 <p className="font-medium">Mutation Success!</p>
                 <pre className="text-xs whitespace-pre-wrap">{JSON.stringify(mutationResult, null, 2)}</pre>
               </div>
             </div>
           </div>
        )}
        
        {mutationError && (
            <div className="mt-6 bg-red-50 border border-red-200 rounded-md p-4">
                <div className="flex">
                    <ExclamationTriangleIcon className="h-5 w-5 text-red-400 mr-2 flex-shrink-0" />
                    <div className="text-sm text-red-700">
                        <p className="font-medium">Error</p>
                        <p>{mutationError}</p>
                    </div>
                </div>
            </div>
        )}

      </div>
    </div>
  );
};

export default DataStorageForm; 