// Key Management Tab wrapper component

import { useState } from 'react';
import { useAppSelector, useAppDispatch } from '../../store/hooks';
import { validatePrivateKey, clearAuthentication } from '../../store/authSlice';
import { ShieldCheckIcon, ClipboardIcon, CheckIcon, KeyIcon, ExclamationTriangleIcon } from '@heroicons/react/24/outline';

function KeyManagementTab({ onResult: _onResult }) {
    // Redux state and dispatch
    const dispatch = useAppDispatch();
    const authState = useAppSelector(state => state.auth);
    const { isAuthenticated, systemPublicKey, systemKeyId, privateKey, isLoading, error: _authError } = authState;

    // privateKey is now stored as base64 string (no conversion needed)
    const privateKeyBase64 = privateKey;
    
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
        <div className="p-4 minimal-card">
            <h2 className="text-xl font-semibold mb-4 text-success">
                <span className="text-secondary">$</span> key-management
            </h2>

            {/* Current System Public Key Display */}
            <div className="minimal-card border-l-4 border-gray-200-blue p-4 mb-6">
                <div className="flex items-start">
                    <ShieldCheckIcon className="h-5 w-5 text-info mr-2 flex-shrink-0 mt-0.5" />
                    <div className="text-sm text-primary flex-1">
                        <p className="font-medium text-info"># Current System Public Key:</p>
                        {isLoading ? (
                            <p className="text-secondary">Loading...</p>
                        ) : systemPublicKey ? (
                            <div className="mt-2">
                                <div className="flex">
                                    <input
                                        type="text"
                                        value={systemPublicKey && systemPublicKey !== 'null' ? systemPublicKey : ''}
                                        readOnly
                                        className="flex-1 px-2 py-1 border border-gray-200 bg-white text-xs font-mono text-primary"
                                    />
                                    <button
                                        onClick={() => copyToClipboard(systemPublicKey, 'system')}
                                        className="px-2 py-1 border border-l-0 border-gray-200 bg-gray-50 hover:bg-gray-50 focus:outline-none"
                                    >
                                        {copiedField === 'system' ? (
                                            <CheckIcon className="h-3 w-3 text-success" />
                                        ) : (
                                            <ClipboardIcon className="h-3 w-3 text-info" />
                                        )}
                                    </button>
                                </div>
                                {systemKeyId && (
                                    <p className="text-xs text-secondary mt-1">Key ID: {systemKeyId}</p>
                                )}
                                {isAuthenticated && (
                                    <p className="text-xs text-success mt-1">🔓 Authenticated - Private key loaded!</p>
                                )}
                            </div>
                        ) : (
                            <p className="text-secondary mt-1">No system public key available.</p>
                        )}
                    </div>
                </div>
            </div>

            {/* Current Private Key Display */}
            {isAuthenticated && privateKeyBase64 && (
                <div className="minimal-card border-l-4 border-green-500 p-4 mb-6">
                    <div className="flex items-start">
                        <KeyIcon className="h-5 w-5 text-success mr-2 flex-shrink-0 mt-0.5" />
                        <div className="text-sm text-primary flex-1">
                            <p className="font-medium text-success"># Current Private Key (Auto-loaded from Node)</p>
                            <p className="mt-1 text-secondary">Your private key has been automatically loaded from the backend node.</p>
                            
                            <div className="mt-3">
                                <div className="flex">
                                    <textarea
                                        value={privateKeyBase64}
                                        readOnly
                                        className="flex-1 px-3 py-2 border border-gray-200 bg-white text-xs font-mono resize-none text-primary"
                                        rows={3}
                                        placeholder="Private key will appear here..."
                                    />
                                    <button
                                        onClick={() => copyToClipboard(privateKeyBase64, 'private')}
                                        className="px-3 py-2 border border-l-0 border-gray-200 bg-gray-50 hover:bg-gray-50 focus:outline-none"
                                        title="Copy private key"
                                    >
                                        {copiedField === 'private' ? (
                                            <CheckIcon className="h-3 w-3 text-success" />
                                        ) : (
                                            <ClipboardIcon className="h-3 w-3 text-success" />
                                        )}
                                    </button>
                                </div>
                                <p className="text-xs text-success mt-1">🔓 Authenticated - Private key loaded from node!</p>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* Private Key Input Section - Only show if not authenticated */}
            {systemPublicKey && !isAuthenticated && !privateKeyBase64 && (
                <div className="minimal-card border-l-4 border-yellow-400 p-4 mb-6">
                    <div className="flex items-start">
                        <KeyIcon className="h-5 w-5 text-warning mr-2 flex-shrink-0 mt-0.5" />
                        <div className="text-sm text-primary flex-1">
                            <p className="font-medium text-warning"># Import Private Key</p>
                            <p className="mt-1 text-secondary">You have a registered public key but no local private key. Enter your private key to restore access.</p>
                            
                            {!showPrivateKeyInput ? (
                                <button
                                    onClick={() => setShowPrivateKeyInput(true)}
                                    className="mt-3 minimal-btn-secondary border-yellow-400 text-warning"
                                >
                                    <KeyIcon className="h-4 w-4 mr-1" />
                                    Import Private Key
                                </button>
                            ) : (
                                <div className="mt-3 space-y-3">
                                    <div>
                                        <label className="block text-xs font-medium text-secondary mb-1">
                                            --private-key (Base64)
                                        </label>
                                        <textarea
                                            value={privateKeyInput}
                                            onChange={(e) => setPrivateKeyInput(e.target.value)}
                                            placeholder="Enter your private key here..."
                                            className="minimal-textarea w-full text-xs"
                                            rows={3}
                                        />
                                    </div>
                                    
                                    {/* Validation Status */}
                                    {privateKeyValidation && (
                                        <div className={`p-2 text-xs ${
                                            privateKeyValidation.valid 
                                                ? 'border-l-4 border-green-500 text-success'
                                                : 'border-l-4 border-red-500 text-error'
                                        }`}>
                                            {privateKeyValidation.valid ? (
                                                <div className="flex items-center">
                                                    <CheckIcon className="h-4 w-4 text-success mr-1" />
                                                    <span>Private key matches system public key!</span>
                                                </div>
                                            ) : (
                                                <div className="flex items-center">
                                                    <ExclamationTriangleIcon className="h-4 w-4 text-error mr-1" />
                                                    <span>{privateKeyValidation.error}</span>
                                                </div>
                                            )}
                                        </div>
                                    )}
                                    
                                    <div className="flex gap-2">
                                        <button
                                            onClick={handlePrivateKeySubmit}
                                            disabled={isValidatingPrivateKey || !privateKeyInput.trim()}
                                            className="minimal-btn-secondary minimal-btn text-xs disabled:opacity-50"
                                        >
                                            {isValidatingPrivateKey ? 'Validating...' : '→ Validate & Import'}
                                        </button>
                                        <button
                                            onClick={handleCancelPrivateKeyInput}
                                            className="minimal-btn-secondary text-xs"
                                        >
                                            Cancel
                                        </button>
                                    </div>
                                    
                                    <div className="minimal-card border-l-4 border-red-500 p-2">
                                        <div className="flex">
                                            <ExclamationTriangleIcon className="h-4 w-4 text-error mr-1 flex-shrink-0" />
                                            <div className="text-xs text-secondary">
                                                <p className="font-medium text-error"># Security Warning:</p>
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