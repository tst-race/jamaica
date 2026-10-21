
// 1. Create a WebSocket connection
// const socket = new WebSocket('ws://your-websocket-server');
// const socket = new WebSocket('https://echo.websocket.org/');
const socket = new WebSocket('ws://localhost:8080');


// 2. Handle the connection opening
socket.addEventListener('open', (event) => {
    console.log('Connected to WebSocket server');
    // Optional: Send data to the server
    socket.send('Hello Server!');
});

// 3. Handle incoming messages
socket.addEventListener('message', (event) => {
    console.log('Message from server:', event.data);
    // You can parse JSON if the server sends JSON data
    // const data = JSON.parse(event.data);
    // console.log('Parsed data:', data);
    data.push(event.data);
    if (data.length > 100) {
        data.shift();
    }
    update();
});

// 4. Handle errors
socket.addEventListener('error', (event) => {
    console.error('WebSocket error:', event);
});

// 5. Handle connection closing
socket.addEventListener('close', (event) => {
    console.log('WebSocket connection closed:', event);
});


// const random = d3.randomNormal(0, 0.2);
var data = [0]; // d3.range(40).map(random);

const page_width = 800;
const page_height = 800;
const margin = {top: 20, right: 30, bottom: 30, left: 40},
      width = page_width - margin.left - margin.right,
      height = page_height - margin.top - margin.bottom;

const svg = d3.select("svg")
    .attr("width", width + margin.left + margin.right)
    .attr("height", height + margin.top + margin.bottom)
    .append("g")
    .attr("transform", `translate(${margin.left},${margin.top})`);

// Set up scales
const x = d3.scaleLinear()
    .domain([0, 100])
    .range([0, width]);

const y = d3.scaleLinear()
    .domain([-1, 1])
    .range([height, 0]);

// Add axes
const gx = svg.append("g")
    .attr("transform", `translate(0, ${height})`)
    .call(d3.axisBottom(x));
// svg.append("g")
//     .attr("transform", `translate(0, ${y(0)})`)
//     .call(d3.axisBottom(x));

svg.append("g")
    .call(d3.axisLeft(y));

const line_packet_size = d3.line()
    .x((d, i) => x(i))
    .y(d => y(d));

svg.append("path")
    .datum(data)
    .attr("class", "line")
    .attr("d", line_packet_size);

function update() {
//     d3.select(this)
//         .attr("transform", "translate(" + x(-1) + ",0)")
//         .transition()
//         .on("start", update);    
    // Update domains.
    // y.domain([0, d3.max(data, d => d)])

    // Update the x axis.
    // x.domain([0, data.length - 1]);
    // gx.call(d3.axisBottom(x));

    // svg.append("g")
    //     .attr("transform", `translate(0, ${height})`)
    //     .call(d3.axisBottom(x));

    svg.select(".line")
        .datum(data)
        .attr("d", line_packet_size);
}
