const WebSocket = require('ws');
const fs = require('fs');
const readline = require('readline');

// Create a WebSocket server on port 8080
const wss = new WebSocket.Server({ port: 8080 });

// Function to broadcast data to all connected clients
function broadcast(data) {
    wss.clients.forEach(client => {
        if (client.readyState === WebSocket.OPEN) {
            client.send(data);
        }
    });
}

// Handle connection events
wss.on('connection', (ws) => {
    console.log('Client connected');

    // Handle when a client disconnects
    ws.on('close', () => {
        console.log('Client disconnected');
    });
});

// Read lines from a file and send them over WebSocket
const fileStream = fs.createReadStream('../../client_bandwidth.csv');
const rl = readline.createInterface({
    input: fileStream,
    crlfDelay: Infinity
});

rl.on('line', (line) => {
    console.log('Sending line:', line);
    broadcast(line);
});

rl.on('close', () => {
    console.log('Finished reading file');
});
