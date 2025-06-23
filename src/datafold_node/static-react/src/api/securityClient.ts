import type { SignedMessage } from '../types/cryptography';
import type {
  ApiResponse,
  VerificationResponse,
  KeyRegistrationResponse,
} from '../types/api';
import type { KeyRegistrationRequest } from '../types/cryptography';
import { get as httpGet, post as httpPost } from '../utils/httpClient';

const API_BASE_URL = '/api/security';

async function post<T>(endpoint: string, body: any): Promise<ApiResponse<T>> {
  return httpPost<T>(API_BASE_URL, endpoint, body);
}

async function get<T>(endpoint: string): Promise<ApiResponse<T>> {
  return httpGet<T>(API_BASE_URL, endpoint);
}

export async function verifyMessage(
  signedMessage: SignedMessage
): Promise<ApiResponse<VerificationResponse>> {
  return post<VerificationResponse>('/verify-message', signedMessage);
}

export async function registerPublicKey(
  request: KeyRegistrationRequest,
): Promise<ApiResponse<KeyRegistrationResponse>> {
  return post<KeyRegistrationResponse>('/system-key', request);
}

export async function getSystemPublicKey(): Promise<ApiResponse<{ public_key: string; public_key_id?: string }>> {
  return get('/system-key');
}
