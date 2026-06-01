/** @type {import('./$types').PageServerLoad} */
export async function load({ fetch }) {
    const res = await fetch('/api/early-access', {
        method: 'POST'
    });
    console.log('Early Access Response Status:', res.status);
    const data = await res.json();
    console.log('Early Access Load Data:', JSON.stringify(data, null, 2));
    return {
        requests: data.data || []
    };
}
