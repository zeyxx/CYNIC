/**
 * Hermes Background Worker
 * Relays engagement signals to the CYNIC Kernel.
 */

const KERNEL_URL = "http://localhost:3030/observe";
const API_KEY = process.env.CYNIC_API_KEY || "<CYNIC_API_KEY>"; // Set via environment

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.type === "HERMES_ENGAGEMENT") {
    const engagement = request.data;
    
    // Forward to CYNIC Kernel
    fetch(KERNEL_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${API_KEY}`
      },
      body: JSON.stringify({
        tool: "hermes-extension",
        target: engagement.tweet_id,
        domain: "twitter-ground-truth",
        status: "success",
        context: JSON.stringify(engagement)
      })
    })
    .then(response => {
      console.log("🐕 Hermes relayed to Kernel:", response.status);
    })
    .catch(error => {
      console.error("🐕 Hermes relay failed (Is Kernel running on 3030?):", error);
    });
  }
});
