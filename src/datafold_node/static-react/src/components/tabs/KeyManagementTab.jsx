// Key Management Tab wrapper component

import { useState, useEffect } from 'react';
import { useKeyLifecycle } from '../../hooks/useKeyLifecycle';
import { useKeyGeneration } from '../../hooks/useKeyGeneration.js';
import { useAppSelector, useAppDispatch } from '../../store/hooks';
import { validatePrivateKey, clearAuthentication, updateSystemKey, refreshSystemKey } from '../../store/authSlice';
import { ShieldCheckIcon, ClipboardIcon, CheckIcon, KeyIcon, ExclamationTriangleIcon } from '@heroicons/react/24/outline';
import { bytesToBase64, base64ToBytes } from '../../utils/ed25519';
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// Set up SHA-512 hash function for ed25519
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

function KeyManagementTab({ onResult, keyGenerationResult }) {
    // Redux state and dispatch
    const dispatch = useAppDispatch();
    const authState = useAppSelector(state => state.auth);
    const { isAuthenticated, systemPublicKey, systemKeyId, isLoading, error: _authError } = authState;
    
    
    const [isRegistering, setIsRegistering] = useState(false);
    const [registrationStatus, setRegistrationStatus] = useState(null); // State for feedback
    const [copiedField, setCopiedField] = useState(null);
    
    // Private key input state
    const [privateKeyInput, setPrivateKeyInput] = useState('');
    const [isValidatingPrivateKey, setIsValidatingPrivateKey] = useState(false);
    const [privateKeyValidation, setPrivateKeyValidation] = useState(null);
    const [showPrivateKeyInput, setShowPrivateKeyInput] = useState(false);

    // Use key generation from prop (shared state) or hook as fallback
    const keyGeneration = keyGenerationResult || useKeyGeneration();
    const { keyPair, isGenerating, error, generateKeys, clearKeys, registerPublicKey } = keyGeneration;
    
    // Extract public key from keyPair if it exists
    const publicKeyBase64 = keyPair?.publicKeyBase64;
    
    // Enhanced lifecycle management with multiple cleanup functions
    useKeyLifecycle([clearKeys, () => dispatch(clearAuthentication())]);

    // Auto-authentication when conditions are met
    useEffect(() => {

        const attemptAutoAuthentication = async () => {
            // Only attempt auto-auth if:
            // 1. We have a local keypair
            // 2. We have a system public key
            // 3. The public keys match
            // 4. We're not already authenticated
            if (keyPair &&
                keyPair.privateKey &&
                publicKeyBase64 &&
                systemPublicKey &&
                publicKeyBase64 === systemPublicKey &&
                !isAuthenticated) {
                
                try {
                    // Wait a moment to ensure React state has updated
                    await new Promise(resolve => setTimeout(resolve, 200));
                    
                    // Attempt authentication directly (no need to refresh system key)
                    const privateKeyBase64 = bytesToBase64(keyPair.privateKey);
                    await dispatch(validatePrivateKey(privateKeyBase64)).unwrap();
                    console.log('Auto-authentication successful in useEffect');
                } catch (error) {
                    // Auto-authentication failed silently
                    console.warn('Auto-authentication failed in useEffect:', error.message);
                }
            }
        };

        attemptAutoAuthentication();
    }, [keyPair, publicKeyBase64, systemPublicKey, isAuthenticated, dispatch]);

    // System public key is now managed by keyAuth hook

    const copyToClipboard = async (text, field) => {
        try {
            await navigator.clipboard.writeText(text);
            setCopiedField(field);
            setTimeout(() => setCopiedField(null), 2000);
        } catch (err) {
            console.error('Failed to copy:', err);
        }
    };

    // Manual cleanup that includes UI state (lifecycle hook handles keys & auth)
    const handleClearKeys = () => {
        clearKeys(); // This triggers useKeyLifecycle cleanup for keys & auth
        clearPrivateKeyInput(); // Clear UI state not handled by lifecycle
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
                // Store the private key in the key generation result state for use by other components
                const privateKeyBytes = base64ToBytes(privateKeyInput.trim());
                const publicKeyBytes = await ed.getPublicKeyAsync(privateKeyBytes);
                
                // Update the parent component's state with the validated key pair
                if (keyGenerationResult && keyGenerationResult.result && keyGenerationResult.setResult) {
                    keyGenerationResult.setResult(prev => ({
                        ...prev,
                        keyPair: {
                            privateKey: privateKeyBytes,
                            publicKey: publicKeyBytes
                        },
                        publicKeyBase64: systemPublicKey,
                        error: null
                    }));
                } else {
                    // Key validation successful - Redux auth state is already updated
                    console.log('Private key validation successful');
                }
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

    // Clear only private key input UI state (no auth clearing - handled by parent)
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

    const handleRegister = async () => {
        if (!publicKeyBase64) return;
        setIsRegistering(true);
        setRegistrationStatus(null); // Clear previous status
        try {
            const success = await registerPublicKey(publicKeyBase64);
            
            const status = {
                success,
                message: success ? 'Public key registered successfully!' : 'Failed to register public key.',
            };
            setRegistrationStatus(status);
            onResult(status); // Also update the main result panel
            
            // Update system key state immediately after successful registration
            if (success) {
                // Update Redux state directly with the key we just registered
                dispatch(updateSystemKey({
                    systemPublicKey: publicKeyBase64,
                    systemKeyId: 'SYSTEM_WIDE_PUBLIC_KEY'
                }));
                console.log('Updated system key state directly after registration');
                
                // If user has a local private key from generation, auto-authenticate
                if (keyPair && keyPair.privateKey) {
                    try {
                        // Small delay to ensure React state has updated
                        await new Promise(resolve => setTimeout(resolve, 200));
                        
                        const privateKeyBase64 = bytesToBase64(keyPair.privateKey);
                        await dispatch(validatePrivateKey(privateKeyBase64)).unwrap();
                        console.log('Auto-authentication successful after registration');
                    } catch (authError) {
                        // Auto-authentication failed, but registration was successful
                        // Don't override the successful registration status
                        console.warn('Auto-authentication failed after successful registration:', authError.message);
                    }
                }
            }
        } catch (e) {
            const status = { success: false, message: e.message };
            setRegistrationStatus(status);
            onResult(status);
        }
        setIsRegistering(false);
    };


    return (
        <div className="p-4 bg-white rounded-lg shadow">
            <h2 className="text-xl font-semibold mb-4">Key Management</h2>
            <div className="flex space-x-2 mb-4">
                <button
                    onClick={generateKeys}
                    disabled={isGenerating}
                    className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:bg-gray-400"
                >
                    {isGenerating ? 'Generating...' : 'Generate New Keypair'}
                </button>
                <button
                    onClick={handleClearKeys}
                    disabled={!publicKeyBase64}
                    className="px-4 py-2 bg-gray-500 text-white rounded hover:bg-gray-600 disabled:bg-gray-400"
                >
                    Clear Keys
                </button>
                <button
                    onClick={handleRegister}
                    disabled={!publicKeyBase64 || isRegistering}
                    className="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 disabled:bg-gray-400"
                >
                    {isRegistering ? 'Registering...' : 'Register Public Key'}
                </button>
            </div>

            {registrationStatus && (
                <div
                    className={`p-4 mb-4 text-sm rounded-lg ${
                        registrationStatus.success
                            ? 'text-green-700 bg-green-100'
                            : 'text-red-700 bg-red-100'
                    }`}
                    role="alert"
                >
                    <span className="font-medium">
                        {registrationStatus.success ? 'Success!' : 'Error:'}
                    </span>{' '}
                    {registrationStatus.message}
                </div>
            )}

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
                                {publicKeyBase64 === systemPublicKey && (
                                    <p className="text-xs text-green-600 mt-1">✅ This matches your newly registered key!</p>
                                )}
                                {isAuthenticated && (
                                    <p className="text-xs text-green-600 mt-1">🔓 Authenticated - Private key loaded!</p>
                                )}
                            </div>
                        ) : (
                            <p className="text-blue-600 mt-1">No system public key registered yet.</p>
                        )}
                    </div>
                </div>
            </div>
{/* Private Key Input Section */}
{systemPublicKey && !keyPair && (
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

            {error && (
                <div className="p-4 mb-4 text-sm text-red-700 bg-red-100 rounded-lg" role="alert">
                    <span className="font-medium">Error:</span> {error}
                </div>
            )}

            {publicKeyBase64 && (
                <div className="space-y-6">
                    {/* Public Key Display */}
                    <div className="key-display">
                        <label className="block text-sm font-medium text-gray-700 mb-2">
                            Public Key (Base64) - Safe to share
                        </label>
                        <div className="flex">
                            <textarea
                                readOnly
                                value={publicKeyBase64}
                                className="flex-1 p-3 border border-gray-300 rounded-l-md bg-gray-50 text-sm font-mono resize-none"
                                rows={3}
                            />
                            <button
                                onClick={() => copyToClipboard(publicKeyBase64, 'generated-public')}
                                className="px-3 py-2 border border-l-0 border-gray-300 rounded-r-md bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                title="Copy public key"
                            >
                                {copiedField === 'generated-public' ? (
                                    <CheckIcon className="h-4 w-4 text-green-600" />
                                ) : (
                                    <ClipboardIcon className="h-4 w-4 text-gray-500" />
                                )}
                            </button>
                        </div>
                    </div>

                    {/* Private Key Display */}
                    {keyPair && (
                        <div className="key-display">
                            <label className="block text-sm font-medium text-red-700 mb-2">
                                Private Key (Base64) - Keep secret!
                            </label>
                            <div className="flex">
                                <textarea
                                    readOnly
                                    value={bytesToBase64(keyPair.privateKey)}
                                    className="flex-1 p-3 border border-red-300 rounded-l-md bg-red-50 text-sm font-mono resize-none"
                                    rows={3}
                                />
                                <button
                                    onClick={() => copyToClipboard(bytesToBase64(keyPair.privateKey), 'generated-private')}
                                    className="px-3 py-2 border border-l-0 border-red-300 rounded-r-md bg-white hover:bg-red-50 focus:outline-none focus:ring-2 focus:ring-red-500"
                                    title="Copy private key"
                                >
                                    {copiedField === 'generated-private' ? (
                                        <CheckIcon className="h-4 w-4 text-green-600" />
                                    ) : (
                                        <ClipboardIcon className="h-4 w-4 text-red-500" />
                                    )}
                                </button>
                            </div>
                            <div className="bg-red-50 border border-red-200 rounded-md p-3 mt-2">
                                <div className="flex">
                                    <ExclamationTriangleIcon className="h-4 w-4 text-red-400 mr-2 flex-shrink-0" />
                                    <div className="text-xs text-red-700">
                                        <p className="font-medium">Security Warning:</p>
                                        <p>Never share your private key. Store it securely and only on trusted devices. Anyone with your private key can impersonate you.</p>
                                    </div>
                                </div>
                            </div>
                        </div>
                    )}
                </div>
            )}

        </div>
    );
}

export default KeyManagementTab;