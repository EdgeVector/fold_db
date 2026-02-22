// Key Management Tab wrapper component

import { useState, useEffect, useRef } from 'react';
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
    const copiedTimeoutRef = useRef(null);

    useEffect(() => {
      return () => { if (copiedTimeoutRef.current) clearTimeout(copiedTimeoutRef.current) };
    }, []);

    // Private key input state
    const [privateKeyInput, setPrivateKeyInput] = useState('');
    const [isValidatingPrivateKey, setIsValidatingPrivateKey] = useState(false);
    const [privateKeyValidation, setPrivateKeyValidation] = useState(null);
    const [showPrivateKeyInput, setShowPrivateKeyInput] = useState(false);

    const copyToClipboard = async (text, field) => {
        try {
            await navigator.clipboard.writeText(text);
            setCopiedField(field);
            if (copiedTimeoutRef.current) clearTimeout(copiedTimeoutRef.current);
            copiedTimeoutRef.current = setTimeout(() => setCopiedField(null), 2000);
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
              // Validation succeeded - no additional action needed
            }
        } catch (error) {
            setPrivateKeyValidation({
                valid: false,
                error: `Validation failed: ${error instanceof Error ? error.message : String(error)}`
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
        <div className="p-6 space-y-4">
            {/* Current System Public Key Display */}
            <div className="card card-info p-4">
                <div className="flex items-start">
                    <ShieldCheckIcon className="h-5 w-5 text-gruvbox-blue mr-2 flex-shrink-0 mt-0.5" />
                    <div className="text-sm text-primary flex-1">
                        <p className="font-medium text-gruvbox-blue">Current System Public Key</p>
                        {isLoading ? (
                            <p className="text-secondary">Loading...</p>
                        ) : systemPublicKey ? (
                            <div className="mt-2">
                                <div className="flex">
                                    <input
                                        type="text"
                                        value={systemPublicKey && systemPublicKey !== 'null' ? systemPublicKey : ''}
                                        readOnly
                                        className="flex-1 px-2 py-1 text-xs font-mono bg-surface border border-border text-primary focus:outline-none focus:border-primary"
                                    />
                                    <button
                                        onClick={() => copyToClipboard(systemPublicKey, 'system')}
                                        className="px-2 py-1 border border-border border-l-0 bg-surface-secondary focus:outline-none cursor-pointer hover:bg-surface transition-colors"
                                    >
                                        {copiedField === 'system' ? (
                                            <CheckIcon className="h-3 w-3 text-gruvbox-green" />
                                        ) : (
                                            <ClipboardIcon className="h-3 w-3 text-gruvbox-blue" />
                                        )}
                                    </button>
                                </div>
                                {systemKeyId && (
                                    <p className="text-xs text-secondary mt-1">Key ID: {systemKeyId}</p>
                                )}
                                {isAuthenticated && (
                                    <p className="text-xs text-gruvbox-green mt-1">🔓 Authenticated - Private key loaded!</p>
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
                <div className="card card-success p-4 ">
                    <div className="flex items-start">
                        <KeyIcon className="h-5 w-5 text-gruvbox-green mr-2 flex-shrink-0 mt-0.5" />
                        <div className="text-sm text-primary flex-1">
                            <p className="font-medium text-gruvbox-green">Current Private Key (Auto-loaded)</p>
                            <p className="mt-1 text-secondary">Your private key has been automatically loaded from the backend node.</p>
                            
                            <div className="mt-3">
                                <div className="flex">
                                    <textarea
                                        value={privateKeyBase64}
                                        readOnly
                                        className="flex-1 px-3 py-2 text-xs font-mono bg-surface border border-border text-primary resize-none focus:outline-none focus:border-primary"
                                        rows={3}
                                        placeholder="Private key will appear here..."
                                    />
                                    <button
                                        onClick={() => copyToClipboard(privateKeyBase64, 'private')}
                                        className="px-3 py-2 border border-border border-l-0 bg-surface-secondary focus:outline-none cursor-pointer hover:bg-surface transition-colors"
                                        title="Copy private key"
                                    >
                                        {copiedField === 'private' ? (
                                            <CheckIcon className="h-3 w-3 text-gruvbox-green" />
                                        ) : (
                                            <ClipboardIcon className="h-3 w-3 text-gruvbox-green" />
                                        )}
                                    </button>
                                </div>
                                <p className="text-xs text-gruvbox-green mt-1">🔓 Authenticated - Private key loaded from node!</p>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* Private Key Input Section - Only show if not authenticated */}
            {systemPublicKey && !isAuthenticated && !privateKeyBase64 && (
                <div className="card card-warning p-4 ">
                    <div className="flex items-start">
                        <KeyIcon className="h-5 w-5 text-gruvbox-yellow mr-2 flex-shrink-0 mt-0.5" />
                        <div className="text-sm text-primary flex-1">
                            <p className="font-medium text-gruvbox-yellow">Import Private Key</p>
                            <p className="mt-1 text-secondary">You have a registered public key but no local private key. Enter your private key to restore access.</p>
                            
                            {!showPrivateKeyInput ? (
                                <button
                                    onClick={() => setShowPrivateKeyInput(true)}
                                    className="btn-secondary mt-3 flex items-center text-gruvbox-yellow"
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
                                            className="w-full px-3 py-2 text-xs font-mono bg-surface border border-border text-primary placeholder-tertiary focus:outline-none focus:border-primary resize-y"
                                            rows={3}
                                        />
                                    </div>
                                    
                                    {/* Validation Status */}
                                    {privateKeyValidation && (
                                        <div className={`card p-2 text-xs ${
                                            privateKeyValidation.valid ? 'card-success text-gruvbox-green' : 'card-error text-gruvbox-red'
                                        }`}>
                                            {privateKeyValidation.valid ? (
                                                <div className="flex items-center">
                                                    <CheckIcon className="h-4 w-4 text-gruvbox-green mr-1" />
                                                    <span>Private key matches system public key!</span>
                                                </div>
                                            ) : (
                                                <div className="flex items-center">
                                                    <ExclamationTriangleIcon className="h-4 w-4 text-gruvbox-red mr-1" />
                                                    <span>{privateKeyValidation.error}</span>
                                                </div>
                                            )}
                                        </div>
                                    )}
                                    
                                    <div className="flex gap-2">
                                        <button
                                            onClick={handlePrivateKeySubmit}
                                            disabled={isValidatingPrivateKey || !privateKeyInput.trim()}
                                            className="px-3 py-1.5 text-xs bg-gruvbox-orange text-surface border-none cursor-pointer hover:bg-gruvbox-yellow transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                        >
                                            {isValidatingPrivateKey ? 'Validating...' : '→ Validate & Import'}
                                        </button>
                                        <button
                                            onClick={handleCancelPrivateKeyInput}
                                            className="px-3 py-1.5 text-xs text-secondary bg-transparent border border-border cursor-pointer hover:border-primary hover:text-primary transition-colors"
                                        >
                                            Cancel
                                        </button>
                                    </div>
                                    
                                    <div className="card card-error p-2">
                                        <div className="flex">
                                            <ExclamationTriangleIcon className="h-4 w-4 text-gruvbox-red mr-1 flex-shrink-0" />
                                            <div className="text-xs text-secondary">
                                                <p className="font-medium text-gruvbox-red">Security Warning</p>
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