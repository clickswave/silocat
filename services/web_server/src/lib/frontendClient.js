import axios from 'axios';

/**
 * Browser-side axios instance. No baseURL: every call is same-origin against the
 * /api/v1 proxy, which attaches the account credential server-side so it never
 * has to reach the browser.
 */
export const FrontendClient = axios.create({});
