// Key Management Tab wrapper component

import { useState } from 'react';
import { useAppSelector, useAppDispatch } from '../../store/hooks';
import { validatePrivateKey, clearAuthentication } from '../../store/authSlice';
import { ShieldCheckIcon, ClipboardIcon, CheckIcon, KeyIcon, ExclamationTriangleIcon } from '@heroicons/react/24/outline';
import { bytesToBase64 } from '../../utils/cryptoUtils';

function KeyManagementTab({ onResult: _onResult }) {
    // Redux state and dispatch
    const dispatch = useAppDispatch();
    const authState = useAppSelector(state => state.auth);
    const { isAuthenticated, systemPublicKey, systemKeyId, privateKey, isLoading, error: _authError } = authState;
    
    // Convert private key to base64 for display
    const privateKeyBase64 = privateKey ? bytesToBase64(privateKey) : null;
    
    const [copiedField, setCopiedField] = useState(null);
    
    // Private key input state
    const [privateKeyInput, setPrivateKeyInput] = useState('');
    const [isValidatingPrivateKey, setIsValidatingPrivateKey] = useState(false);
    const [privateKeyValidation, setPrivateKeyValidation] = useState(null);
    const [showPrivateKeyInput, setShowPrivateKeyInput] = useState(false);

    const copyToClipboard = async (text, field) => {
        try {
            await navigator.clipboard.writeText(text);
            setCopiedField(field);
            setTimeout(() => setCopiedField(null), 2000);
        } catch (err) {
            console.error('Failed to copy:', err);
        }
    };

    const handlePrivateKeySubmit = async () => {
        if (!privateKeyInput.trim()) {
            setPrivateKeyValidation({ valid: false, error: 'Please enter a private key' });
            return;
        }

        setIsValidatingPrivateKey(true);
        try {
            // Use Redux validatePrivateKey action
            const result = await dispatch(validatePrivateKey(privateKeyInput.trim())).unwrap();
            const isValid = result.isAuthenticated;
            
            setPrivateKeyValidation({
                valid: isValid,
                error: isValid ? null : 'Private key does not match the system public key'
            });
            
            if (isValid) {
                console.log('Private key validation successful');
            }
        } catch (error) {
            setPrivateKeyValidation({
                valid: false,
                error: `Validation failed: ${error.message}`
            });
        } finally {
            setIsValidatingPrivateKey(false);
        }
    };

    // Clear only private key input UI state
    const clearPrivateKeyInput = () => {
        setPrivateKeyInput('');
        setPrivateKeyValidation(null);
        setShowPrivateKeyInput(false);
    };

    // Cancel private key input and clear authentication
    const handleCancelPrivateKeyInput = () => {
        clearPrivateKeyInput();
        dispatch(clearAuthentication());
    };

    return (
        <div className="p-4 bg-white rounded-lg shadow">
            <h2 className="text-xl font-semibold mb-4">Key Management</h2>

            {/* Current System Public Key Display */}
            <div className="bg-blue-50 border border-blue-200 rounded-md p-4 mb-6">
                <div className="flex items-start">
                    <ShieldCheckIcon className="h-5 w-5 text-blue-400 mr-2 flex-shrink-0 mt-0.5" />
                    <div className="text-sm text-blue-700 flex-1">
                        <p className="font-medium">Current System Public Key:</p>
                        {isLoading ? (
                            <p className="text-blue-600">Loading...</p>
                        ) : systemPublicKey ? (
                            <div className="mt-2">
                                <div className="flex">
                                    <input
                                        type="text"
                                        value={systemPublicKey && systemPublicKey !== 'null' ? systemPublicKey : ''}
                                        readOnly
                                        className="flex-1 px-2 py-1 border border-blue-300 rounded-l-md bg-blue-50 text-xs font-mono"
                                    />
                                    <button
                                        onClick={() => copyToClipboard(systemPublicKey, 'system')}
                                        className="px-2 py-1 border border-l-0 border-blue-300 rounded-r-md bg-white hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    >
                                        {copiedField === 'system' ? (
                                            <CheckIcon className="h-3 w-3 text-green-600" />
                                        ) : (
                                            <ClipboardIcon className="h-3 w-3 text-blue-500" />
                                        )}
                                    </button>
                                </div>
                                {systemKeyId && (
                                    <p className="text-xs text-blue-600 mt-1">Key ID: {systemKeyId}</p>
                                )}
                                {isAuthenticated && (
                                    <p className="text-xs text-green-600 mt-1">🔓 Authenticated - Private key loaded!</p>
                                )}
                            </div>
                        ) : (
                            <p className="text-blue-600 mt-1">No system public key available.</p>
                        )}
                    </div>
                </div>
            </div>

            {/* Current Private Key Display */}
            {isAuthenticated && privateKeyBase64 && (
                <div className="bg-green-50 border border-green-200 rounded-md p-4 mb-6">
                    <div className="flex items-start">
                        <KeyIcon className="h-5 w-5 text-green-400 mr-2 flex-shrink-0 mt-0.5" />
                        <div className="text-sm text-green-700 flex-1">
                            <p className="font-medium">Current Private Key (Auto-loaded from Node)</p>
                            <p className="mt-1">Your private key has been automatically loaded from the backend node.</p>
                            
                            <div className="mt-3">
                                <div className="flex">
                                    <textarea
                                        value={privateKeyBase64}
                                        readOnly
                                        className="flex-1 px-3 py-2 border border-green-300 rounded-l-md bg-green-50 text-xs font-mono resize-none"
                                        rows={3}
                                        placeholder="Private key will appear here..."
                                    />
                                    <button
                                        onClick={() => copyToClipboard(privateKeyBase64, 'private')}
                                        className="px-3 py-2 border border-l-0 border-green-300 rounded-r-md bg-white hover:bg-green-50 focus:outline-none focus:ring-2 focus:ring-green-500"
                                        title="Copy private key"
                                    >
                                        {copiedField === 'private' ? (
                                            <CheckIcon className="h-3 w-3 text-green-600" />
                                        ) : (
                                            <ClipboardIcon className="h-3 w-3 text-green-500" />
                                        )}
                                    </button>
                                </div>
                                <p className="text-xs text-green-600 mt-1">🔓 Authenticated - Private key loaded from node!</p>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* Private Key Input Section - Only show if not authenticated */}
            {systemPublicKey && !isAuthenticated && !privateKeyBase64 && (
                <div className="bg-yellow-50 border border-yellow-200 rounded-md p-4 mb-6">
                    <div className="flex items-start">
                        <KeyIcon className="h-5 w-5 text-yellow-400 mr-2 flex-shrink-0 mt-0.5" />
                        <div className="text-sm text-yellow-700 flex-1">
                            <p className="font-medium">Import Private Key</p>
                            <p className="mt-1">You have a registered public key but no local private key. Enter your private key to restore access.</p>
                            
                            {!showPrivateKeyInput ? (
                                <button
                                    onClick={() => setShowPrivateKeyInput(true)}
                                    className="mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-yellow-600 hover:bg-yellow-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500"
                                >
                                    <KeyIcon className="h-4 w-4 mr-1" />
                                    Import Private Key
                                </button>
                            ) : (
                                <div className="mt-3 space-y-3">
                                    <div>
                                        <label className="block text-xs font-medium text-yellow-700 mb-1">
                                            Private Key (Base64)
                                        </label>
                                        <textarea
                                            value={privateKeyInput}
                                            onChange={(e) => setPrivateKeyInput(e.target.value)}
                                            placeholder="Enter your private key here..."
                                            className="w-full px-3 py-2 border border-yellow-300 rounded-md focus:outline-none focus:ring-2 focus:ring-yellow-500 text-xs font-mono"
                                            rows={3}
                                        />
                                    </div>
                                    
                                    {/* Validation Status */}
                                    {privateKeyValidation && (
                                        <div className={`p-2 rounded-md text-xs ${
                                            privateKeyValidation.valid 
                                                ? 'bg-green-50 border border-green-200 text-green-700'
                                                : 'bg-red-50 border border-red-200 text-red-700'
                                        }`}>
                                            {privateKeyValidation.valid ? (
                                                <div className="flex items-center">
                                                    <CheckIcon className="h-4 w-4 text-green-600 mr-1" />
                                                    <span>Private key matches system public key!</span>
                                                </div>
                                            ) : (
                                                <div className="flex items-center">
                                                    <ExclamationTriangleIcon className="h-4 w-4 text-red-600 mr-1" />
                                                    <span>{privateKeyValidation.error}</span>
                                                </div>
                                            )}
                                        </div>
                                    )}
                                    
                                    <div className="flex gap-2">
                                        <button
                                            onClick={handlePrivateKeySubmit}
                                            disabled={isValidatingPrivateKey || !privateKeyInput.trim()}
                                            className="inline-flex items-center px-3 py-2 border border-transparent text-xs font-medium rounded-md shadow-sm text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50"
                                        >
                                            {isValidatingPrivateKey ? 'Validating...' : 'Validate & Import'}
                                        </button>
                                        <button
                                            onClick={handleCancelPrivateKeyInput}
                                            className="inline-flex items-center px-3 py-2 border border-gray-300 text-xs font-medium rounded-md shadow-sm text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500"
                                        >
                                            Cancel
                                        </button>
                                    </div>
                                    
                                    <div className="bg-red-50 border border-red-200 rounded-md p-2">
                                        <div className="flex">
                                            <ExclamationTriangleIcon className="h-4 w-4 text-red-400 mr-1 flex-shrink-0" />
                                            <div className="text-xs text-red-700">
                                                <p className="font-medium">Security Warning:</p>
                                                <p>Only enter your private key on trusted devices. Never share or store private keys in plain text.</p>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

export default KeyManagementTab;