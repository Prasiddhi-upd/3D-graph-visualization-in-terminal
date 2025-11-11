import * as THREE from 'three';
import ForceGraph3D from '3d-force-graph';
import { read } from 'graphlib-dot';
import { PointerLockControls } from 'three/examples/jsm/controls/PointerLockControls.js';

let Graph;
let lastDotText = '';
let instancedMesh;
let colorAttr;
let dummy = new THREE.Object3D();
let clusterMeshes = [];
let currentCluster = null;
let clusterSelectionMode = true;
let colorMode = 'cluster';
let freeCameraControls;
let freeCameraEnabled = false;

// Frustum Culling Variables
let frustum = new THREE.Frustum();
let projectionMatrix = new THREE.Matrix4();
let viewMatrix = new THREE.Matrix4();
let visibleNodeIndices = new Set();
let cullingEnabled = true;
let cullingUpdateInterval = 100;
let lastCullingUpdate = 0;

// ============================================
// ENHANCED LOUVAIN-INSPIRED CLUSTERING
// ============================================
function clusterNodes(nodes, links) {
    console.log("🔍 Starting advanced clustering...");
    
    // Build weighted adjacency list
    const adj = new Map();
    const nodeDegree = new Map();
    
    nodes.forEach(node => {
        adj.set(node.id, []);
        nodeDegree.set(node.id, 0);
    });
    
    links.forEach(link => {
        const sourceId = typeof link.source === 'object' ? link.source.id : link.source;
        const targetId = typeof link.target === 'object' ? link.target.id : link.target;
        const weight = link.weight || 0.5;
        
        if (adj.has(sourceId) && adj.has(targetId)) {
            adj.get(sourceId).push({ node: targetId, weight });
            adj.get(targetId).push({ node: sourceId, weight });
            nodeDegree.set(sourceId, nodeDegree.get(sourceId) + weight);
            nodeDegree.set(targetId, nodeDegree.get(targetId) + weight);
        }
    });

    // Initialize communities (each node starts in its own)
    const communities = new Map();
    nodes.forEach((node, i) => {
        communities.set(node.id, i);
        node.cluster = i;
    });
    
    // Phase 1: Modularity optimization (Louvain-like)
    let improved = true;
    let iteration = 0;
    const maxIterations = 15;
    
    while (improved && iteration < maxIterations) {
        improved = false;
        iteration++;
        
        // Shuffle for randomness
        const shuffledNodes = [...nodes].sort(() => Math.random() - 0.5);
        
        for (const node of shuffledNodes) {
            const currentComm = communities.get(node.id);
            const neighbors = adj.get(node.id) || [];
            
            // Calculate weighted connections to each neighboring community
            const commWeights = new Map();
            
            neighbors.forEach(({ node: neighborId, weight }) => {
                const neighborComm = communities.get(neighborId);
                if (neighborComm !== undefined) {
                    commWeights.set(neighborComm, (commWeights.get(neighborComm) || 0) + weight);
                }
            });
            
            // Find best community (highest total weight)
            let bestComm = currentComm;
            let bestWeight = commWeights.get(currentComm) || 0;
            
            for (const [comm, weight] of commWeights) {
                // Only move if significantly better (to avoid oscillation)
                if (weight > bestWeight * 1.1) {
                    bestWeight = weight;
                    bestComm = comm;
                }
            }
            
            if (bestComm !== currentComm) {
                communities.set(node.id, bestComm);
                node.cluster = bestComm;
                improved = true;
            }
        }
        
        console.log(`  Iteration ${iteration}: ${new Set(communities.values()).size} communities`);
    }
    
    // Phase 2: Merge small clusters into nearest neighbors
    const clusterGroups = new Map();
    nodes.forEach(node => {
        const clusterId = communities.get(node.id);
        if (!clusterGroups.has(clusterId)) {
            clusterGroups.set(clusterId, []);
        }
        clusterGroups.get(clusterId).push(node);
    });
    
    const minClusterSize = 3; // Merge clusters smaller than this
    const smallClusters = [];
    
    clusterGroups.forEach((nodeList, clusterId) => {
        if (nodeList.length < minClusterSize) {
            smallClusters.push({ id: clusterId, nodes: nodeList });
        }
    });
    
    // Merge small clusters
    smallClusters.forEach(smallCluster => {
        let bestTarget = null;
        let maxConnection = -1;
        
        // Find the cluster with strongest connection
        clusterGroups.forEach((targetNodes, targetId) => {
            if (targetId === smallCluster.id || targetNodes.length < minClusterSize) return;
            
            let connectionWeight = 0;
            smallCluster.nodes.forEach(node => {
                const neighbors = adj.get(node.id) || [];
                neighbors.forEach(({ node: neighborId, weight }) => {
                    const neighborNode = nodes.find(n => n.id === neighborId);
                    if (neighborNode && communities.get(neighborNode.id) === targetId) {
                        connectionWeight += weight;
                    }
                });
            });
            
            if (connectionWeight > maxConnection) {
                maxConnection = connectionWeight;
                bestTarget = targetId;
            }
        });
        
        // Merge into best target
        if (bestTarget !== null) {
            smallCluster.nodes.forEach(node => {
                communities.set(node.id, bestTarget);
                node.cluster = bestTarget;
            });
            
            clusterGroups.get(bestTarget).push(...smallCluster.nodes);
            clusterGroups.delete(smallCluster.id);
        }
    });
    
    // Phase 3: Renumber clusters sequentially
    const finalClusterMap = new Map();
    let clusterId = 0;
    
    clusterGroups.forEach((nodeList, oldId) => {
        finalClusterMap.set(oldId, clusterId++);
    });
    
    nodes.forEach(node => {
        const oldCluster = communities.get(node.id);
        node.cluster = finalClusterMap.get(oldCluster);
    });
    
    // Build final cluster array
    const clusters = [];
    const finalGroups = new Map();
    
    nodes.forEach(node => {
        if (!finalGroups.has(node.cluster)) {
            finalGroups.set(node.cluster, []);
        }
        finalGroups.get(node.cluster).push(node);
    });
    
    finalGroups.forEach((nodeList, id) => {
        clusters.push({
            id,
            nodes: nodeList,
            size: nodeList.length
        });
    });
    
    // Sort by size for better visualization
    clusters.sort((a, b) => b.size - a.size);
    
    console.log(`✅ Created ${clusters.length} clusters`);
    console.log(`📊 Cluster sizes:`, clusters.map(c => c.size).join(', '));
    
    return clusters;
}

// ---------------- convertGraph() ----------------
function convertGraph(graph) {
    const nodes = graph.nodes().map((id, i) => {
        const attrs = graph.node(id) || {};
        return {
            id,
            label: attrs.label || id,
            color: attrs.fillcolor || attrs.color || '#666666',
            x: (Math.random() - 0.5) * 100,
            y: (Math.random() - 0.5) * 100,
            z: (Math.random() - 0.5) * 100,
            index: i,
            ...attrs
        };
    });

    const links = graph.edges().map(edge => {
        const edgeAttrs = graph.edge(edge) || {};
        return {
            source: edge.v,
            target: edge.w,
            weight: parseFloat(edgeAttrs.label) || 0.5,
            penwidth: parseFloat(edgeAttrs.penwidth) || 1
        };
    });

    // Perform clustering
    const clusters = clusterNodes(nodes, links);
    
    // Assign cluster-based positions
    assignClusterPositions(nodes, clusters);
    
    return { nodes, links };
}

// ---------------- Assign Cluster Positions ----------------
function assignClusterPositions(nodes, clusters) {
    const numClusters = clusters.length;
    
    // Use spherical arrangement for clusters
    const goldenRatio = (1 + Math.sqrt(5)) / 2;
    const angleIncrement = Math.PI * 2 * goldenRatio;
    const radius = 2000; // Distance from origin
    
    clusters.forEach((cluster, index) => {
        // Fibonacci sphere distribution for even spacing
        const t = index / numClusters;
        const inclination = Math.acos(1 - 2 * t);
        const azimuth = angleIncrement * index;
        
        const clusterRadius = Math.max(150, Math.min(500, Math.sqrt(cluster.size) * 40));
        
        const center = {
            x: Math.sin(inclination) * Math.cos(azimuth) * radius,
            y: Math.cos(inclination) * radius,
            z: Math.sin(inclination) * Math.sin(azimuth) * radius
        };
        
        // Position nodes in a sphere around cluster center
        cluster.nodes.forEach((node, nodeIndex) => {
            const nodeFraction = nodeIndex / cluster.nodes.length;
            const nodeInclination = Math.acos(1 - 2 * nodeFraction);
            const nodeAzimuth = angleIncrement * nodeIndex;
            const nodeDistance = Math.random() * clusterRadius;
            
            node.x = center.x + Math.sin(nodeInclination) * Math.cos(nodeAzimuth) * nodeDistance;
            node.y = center.y + Math.cos(nodeInclination) * nodeDistance;
            node.z = center.z + Math.sin(nodeInclination) * Math.sin(nodeAzimuth) * nodeDistance;
            node.clusterCenter = center;
            node.clusterRadius = clusterRadius;
        });
    });
}

// ---------------- Cluster Colors ----------------
function getClusterColor(clusterId, totalClusters) {
    const goldenRatio = 0.618033988749895;
    const hue = (clusterId * goldenRatio * 360) % 360;
    const saturation = 65 + Math.random() * 25;
    const lightness = 50 + Math.random() * 20;
    return `hsl(${hue}, ${saturation}%, ${lightness}%)`;
}

function getWeightColor(weight) {
    const intensity = Math.min(1, weight / 1.0);
    return `hsl(${240 - intensity * 120}, 80%, ${50 + intensity * 30}%)`;
}

function getDegreeColor(degree, maxDegree) {
    const intensity = degree / Math.max(maxDegree, 1);
    return `hsl(120, 80%, ${30 + intensity * 50}%)`;
}

// ---------------- Frustum Culling Functions ----------------
function updateFrustum() {
    const camera = Graph.camera();
    camera.updateMatrixWorld();
    projectionMatrix.copy(camera.projectionMatrix);
    viewMatrix.copy(camera.matrixWorldInverse);
    frustum.setFromProjectionMatrix(new THREE.Matrix4().multiplyMatrices(projectionMatrix, viewMatrix));
}

function performFrustumCulling(nodes) {
    if (!cullingEnabled || !instancedMesh) return;
    updateFrustum();
    visibleNodeIndices.clear();
    const sphere = new THREE.Sphere();
    const nodeRadius = 8;
    
    nodes.forEach((node, index) => {
        sphere.set(new THREE.Vector3(node.x, node.y, node.z), nodeRadius);
        if (frustum.intersectsSphere(sphere)) {
            visibleNodeIndices.add(index);
        }
    });
    updateNodeVisibility();
}

function updateNodeVisibility() {
    if (!instancedMesh || !colorAttr) return;
    const nodes = Graph.graphData().nodes;
    if (!nodes) return;
    
    nodes.forEach((node, i) => {
        const visible = visibleNodeIndices.has(i);
        dummy.position.set(node.x, node.y, node.z);
        dummy.scale.setScalar(visible ? 1 : 0.001);
        dummy.updateMatrix();
        instancedMesh.setMatrixAt(i, dummy.matrix);
    });
    
    instancedMesh.instanceMatrix.needsUpdate = true;
    if (instancedMesh.instanceColor) {
        instancedMesh.instanceColor.needsUpdate = true;
    }
}

function toggleFrustumCulling() {
    cullingEnabled = !cullingEnabled;
    
    if (!cullingEnabled) {
        const nodes = Graph.graphData().nodes;
        if (nodes && instancedMesh) {
            nodes.forEach((node, i) => {
                dummy.position.set(node.x, node.y, node.z);
                dummy.scale.setScalar(1);
                dummy.updateMatrix();
                instancedMesh.setMatrixAt(i, dummy.matrix);
            });
            instancedMesh.instanceMatrix.needsUpdate = true;
        }
    }
    
    console.log(`Frustum culling ${cullingEnabled ? 'enabled' : 'disabled'}`);
    updateCullingStatusDisplay();
}

function updateCullingStatusDisplay() {
    let cullingStatusElement = document.getElementById('culling-status');
    if (!cullingStatusElement) {
        cullingStatusElement = document.createElement('div');
        cullingStatusElement.id = 'culling-status';
        cullingStatusElement.style.cssText = `
            position: absolute; top: 110px; left: 10px; padding: 10px;
            background-color: #666; color: white; border-radius: 5px;
            font-family: Arial, sans-serif; z-index: 1000;
        `;
        document.body.appendChild(cullingStatusElement);
    }
    cullingStatusElement.textContent = `Frustum Culling: ${cullingEnabled ? 'ON' : 'OFF'} (Press 'F')`;
    cullingStatusElement.style.backgroundColor = cullingEnabled ? '#4CAF50' : '#f44336';
}

// ---------------- Create Cluster Clickable Areas ----------------
function createClusterClickableAreas(nodes) {
    clusterMeshes.forEach(mesh => Graph.scene().remove(mesh));
    clusterMeshes = [];
    
    const clusters = {};
    nodes.forEach(node => {
        if (node.cluster !== undefined) {
            if (!clusters[node.cluster]) clusters[node.cluster] = [];
            clusters[node.cluster].push(node);
        }
    });
    
    Object.keys(clusters).forEach(clusterId => {
        const clusterNodes = clusters[clusterId];
        
        let minX = Infinity, maxX = -Infinity;
        let minY = Infinity, maxY = -Infinity;
        let minZ = Infinity, maxZ = -Infinity;
        
        clusterNodes.forEach(node => {
            minX = Math.min(minX, node.x); maxX = Math.max(maxX, node.x);
            minY = Math.min(minY, node.y); maxY = Math.max(maxY, node.y);
            minZ = Math.min(minZ, node.z); maxZ = Math.max(maxZ, node.z);
        });
        
        const center = {
            x: (minX + maxX) / 2,
            y: (minY + maxY) / 2,
            z: (minZ + maxZ) / 2
        };
        
        const radius = Math.max((maxX - minX) / 2, (maxY - minY) / 2, (maxZ - minZ) / 2) * 2.0;
        
        const geometry = new THREE.SphereGeometry(radius, 16, 12);
        const material = new THREE.MeshBasicMaterial({ visible: false, transparent: true, opacity: 0.0 });
        
        const clusterMesh = new THREE.Mesh(geometry, material);
        clusterMesh.position.set(center.x, center.y, center.z);
        clusterMesh.userData = {
            isCluster: true,
            clusterId: parseInt(clusterId),
            nodes: clusterNodes
        };
        
        Graph.scene().add(clusterMesh);
        clusterMeshes.push(clusterMesh);
    });
}

// ---------------- Handle Cluster Click ----------------
function handleClusterClick(clusterId, clusterNodes) {
    currentCluster = clusterId;
    clusterSelectionMode = false;
    
    console.log(`Selected cluster ${clusterId} with ${clusterNodes.length} nodes`);
    
    const center = clusterNodes.reduce((acc, node) => ({
        x: acc.x + node.x, y: acc.y + node.y, z: acc.z + node.z
    }), { x: 0, y: 0, z: 0 });
    
    center.x /= clusterNodes.length;
    center.y /= clusterNodes.length;
    center.z /= clusterNodes.length;
    
    const clusterSize = Math.max(150, clusterNodes.length * 3);
    const cameraDistance = clusterSize * 2.5;
    
    Graph.cameraPosition(
        { x: center.x + cameraDistance * 0.7, y: center.y + cameraDistance * 0.5, z: center.z + cameraDistance * 0.7 },
        center, 1000
    );
    
    updateUIState();
}

// ---------------- Handle Node Click ----------------
function handleNodeClick(node) {
    if (clusterSelectionMode) return;
    
    console.log(`Selected node: ${node.id} in cluster ${node.cluster}`);
    
    const distance = 100;
    const distRatio = 1 + distance / Math.hypot(node.x, node.y, node.z);
    const newPos = node.x || node.y || node.z
        ? { x: node.x * distRatio, y: node.y * distRatio, z: node.z * distRatio }
        : { x: 0, y: 0, z: distance };
    
    Graph.cameraPosition(newPos, node, 1000);
}

// ---------------- Update UI State ----------------
function updateUIState() {
    const statusElement = document.getElementById('graph-status') || createStatusElement();
    const colorModeElement = document.getElementById('color-mode-display') || createColorModeDisplay();
    
    if (clusterSelectionMode) {
        statusElement.textContent = 'Mode: Cluster Selection - Click a cluster';
        statusElement.style.backgroundColor = '#4CAF50';
    } else {
        statusElement.textContent = `Mode: Cluster ${currentCluster} - Click nodes to inspect`;
        statusElement.style.backgroundColor = '#2196F3';
    }
    
    colorModeElement.textContent = `Color: ${colorMode.charAt(0).toUpperCase() + colorMode.slice(1)} (Press 'C')`;
}

function createStatusElement() {
    const el = document.createElement('div');
    el.id = 'graph-status';
    el.style.cssText = `position: absolute; top: 10px; left: 10px; padding: 10px;
        background-color: #4CAF50; color: white; border-radius: 5px;
        font-family: Arial, sans-serif; z-index: 1000;`;
    document.body.appendChild(el);
    return el;
}

function createColorModeDisplay() {
    const el = document.createElement('div');
    el.id = 'color-mode-display';
    el.style.cssText = `position: absolute; top: 60px; left: 10px; padding: 10px;
        background-color: #666; color: white; border-radius: 5px;
        font-family: Arial, sans-serif; z-index: 1000;`;
    document.body.appendChild(el);
    return el;
}

// ---------------- Back to Cluster Selection ----------------
function backToClusterSelection() {
    currentCluster = null;
    clusterSelectionMode = true;
    Graph.cameraPosition({ x: 0, y: 0, z: 3000 }, { x: 0, y: 0, z: 0 }, 1500);
    updateUIState();
}

// ---------------- Color Mode Functions ----------------
function toggleColorMode() {
    const modes = ['cluster', 'weight', 'degree'];
    colorMode = modes[(modes.indexOf(colorMode) + 1) % modes.length];
    console.log(`Color mode: ${colorMode}`);
    updateNodeColors();
    updateUIState();
}

function updateNodeColors() {
    const nodes = Graph.graphData().nodes;
    if (!nodes || nodes.length === 0) return;

    const totalClusters = new Set(nodes.map(n => n.cluster)).size;
    const maxDegree = Math.max(...nodes.map(n => 
        Graph.graphData().links.filter(link => 
            link.source.id === n.id || link.target.id === n.id
        ).length
    ));

    nodes.forEach(node => {
        node.color = getNodeColorBasedOnMode(node, totalClusters, maxDegree);
        node.__threeObj = null;
    });
    
    if (instancedMesh && colorAttr) {
        nodes.forEach((node, i) => {
            const color = new THREE.Color(node.color);
            colorAttr.setXYZ(i, color.r, color.g, color.b);
        });
        instancedMesh.instanceColor.needsUpdate = true;
        colorAttr.needsUpdate = true;
    }
    
    if (Graph) Graph.refresh();
}

function getNodeColorBasedOnMode(node, totalClusters, maxDegree) {
    switch (colorMode) {
        case 'cluster':
            return getClusterColor(node.cluster || 0, totalClusters);
        case 'weight':
            const links = Graph.graphData().links;
            const connectedLinks = links.filter(link => 
                link.source.id === node.id || link.target.id === node.id
            );
            const avgWeight = connectedLinks.reduce((sum, link) => sum + link.weight, 0) / Math.max(connectedLinks.length, 1);
            return getWeightColor(avgWeight);
        case 'degree':
            const degree = Graph.graphData().links.filter(link => 
                link.source.id === node.id || link.target.id === node.id
            ).length;
            return getDegreeColor(degree, maxDegree);
        default:
            return getClusterColor(node.cluster || 0, totalClusters);
    }
}

// ---------------- Update Graph ----------------
function updateGraph(newData) {
    Graph.graphData(newData);
    createClusterClickableAreas(newData.nodes);
    
    const numClusters = new Set(newData.nodes.map(node => node.cluster)).size;
    console.log(`Visualizing ${numClusters} clusters`);
    
    // Enhanced force configuration
    Graph.d3Force('charge').strength(-200).distanceMax(1000);
    Graph.d3Force('center').strength(0.01);
    Graph.d3Force('link')
        .distance(link => 80 * (1.5 - (link.weight || 0.5)))
        .strength(link => (link.weight || 0.3) * 0.9);

    // Cluster cohesion and separation force
    Graph.d3Force('clusterForce', (alpha) => {
        const nodes = Graph.graphData().nodes;
        const clusterCenters = {};
        
        nodes.forEach(node => {
            if (!clusterCenters[node.cluster]) {
                clusterCenters[node.cluster] = { x: 0, y: 0, z: 0, count: 0 };
            }
            clusterCenters[node.cluster].x += node.x;
            clusterCenters[node.cluster].y += node.y;
            clusterCenters[node.cluster].z += node.z;
            clusterCenters[node.cluster].count++;
        });
        
        Object.keys(clusterCenters).forEach(cluster => {
            const data = clusterCenters[cluster];
            data.x /= data.count;
            data.y /= data.count;
            data.z /= data.count;
        });
        
        // Inter-cluster repulsion
        const clusterKeys = Object.keys(clusterCenters);
        for (let i = 0; i < clusterKeys.length; i++) {
            for (let j = i + 1; j < clusterKeys.length; j++) {
                const clusterA = clusterCenters[clusterKeys[i]];
                const clusterB = clusterCenters[clusterKeys[j]];
                
                const dx = clusterB.x - clusterA.x;
                const dy = clusterB.y - clusterA.y;
                const dz = clusterB.z - clusterA.z;
                const distance = Math.sqrt(dx * dx + dy * dy + dz * dz);
                
                const minDistance = 1500;
                if (distance < minDistance) {
                    const force = (minDistance - distance) / minDistance * 500 * alpha;
                    const forceX = (dx / distance) * force;
                    const forceY = (dy / distance) * force;
                    const forceZ = (dz / distance) * force;
                    
                    nodes.forEach(node => {
                        if (node.cluster === parseInt(clusterKeys[i])) {
                            node.x -= forceX; node.y -= forceY; node.z -= forceZ;
                        } else if (node.cluster === parseInt(clusterKeys[j])) {
                            node.x += forceX; node.y += forceY; node.z += forceZ;
                        }
                    });
                }
            }
        }

        // Intra-cluster cohesion
        nodes.forEach(node => {
            if (node.clusterCenter) {
                const dx = node.clusterCenter.x - node.x;
                const dy = node.clusterCenter.y - node.y;
                const dz = node.clusterCenter.z - node.z;
                const dist = Math.sqrt(dx * dx + dy * dy + dz * dz);
                
                if (dist > (node.clusterRadius || 300)) {
                    const force = 0.03 * alpha;
                    node.x += dx * force; node.y += dy * force; node.z += dz * force;
                }
            }
        });
    });
    
    Graph.d3VelocityDecay(0.4);
    setTimeout(() => {
        Graph.d3ReheatSimulation();
        setTimeout(() => Graph.d3ReheatSimulation(), 2000);
        setTimeout(() => Graph.d3ReheatSimulation(), 4000);
    }, 500);

    updateNodeColors();

    if (instancedMesh) Graph.scene().remove(instancedMesh);

    const { nodes } = newData;
    const geometry = new THREE.SphereGeometry(5, 8, 8);
    const material = new THREE.MeshBasicMaterial({ vertexColors: true });

    instancedMesh = new THREE.InstancedMesh(geometry, material, nodes.length);
    colorAttr = new THREE.InstancedBufferAttribute(new Float32Array(nodes.length * 3), 3);

    nodes.forEach((node, i) => {
        dummy.position.set(node.x, node.y, node.z);
        dummy.scale.setScalar(1);
        dummy.updateMatrix();
        instancedMesh.setMatrixAt(i, dummy.matrix);

        const color = new THREE.Color(node.color);
        colorAttr.setXYZ(i, color.r, color.g, color.b);
    });

    instancedMesh.instanceColor = colorAttr;
    instancedMesh.frustumCulled = false;
    Graph.scene().add(instancedMesh);
    
    updateUIState();
    updateCullingStatusDisplay();
    performFrustumCulling(nodes);
}

// ---------------- Animation Loop for Culling ----------------
function animateCulling() {
    const now = Date.now();
    if (now - lastCullingUpdate > cullingUpdateInterval) {
        const nodes = Graph.graphData().nodes;
        if (nodes && cullingEnabled) performFrustumCulling(nodes);
        lastCullingUpdate = now;
    }
    requestAnimationFrame(animateCulling);
}

// ---------------- Poll DOT Files ----------------
async function pollDotFiles(url, interval = 60000) {
    try {
        const res = await fetch(url, { cache: "no-store" });
        const newDotText = await res.text();

        if (newDotText !== lastDotText) {
            lastDotText = newDotText;
            const graph = read(newDotText);
            const { nodes, links } = convertGraph(graph);
            updateGraph({ nodes, links });
        }
    } catch (err) {
        console.error('Failed to fetch or parse DOT:', err);
    }
    setTimeout(() => pollDotFiles(url, interval), interval);
}

// ---------------- ForceGraph3D Setup ----------------
const elem = document.getElementById('3d-graph');
Graph = ForceGraph3D()(elem)
    .nodeLabel('id')
    .linkDirectionalParticles(0)
    .linkWidth(link => (link.weight || 0.5) * 3)
    .linkOpacity(0.6)
    .linkColor(link => {
        const sourceNode = typeof link.source === 'object' ? link.source : 
            Graph.graphData().nodes.find(n => n.id === link.source);
        return sourceNode?.color || '#666666';
    })
    .nodeColor(node => node.color || '#ccc')
    .nodeRelSize(6)
    .linkMaterial(() => new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.6 }))
    .onNodeClick((node, event) => {
        event.stopPropagation();
        handleNodeClick(node);
    });

Graph.onBackgroundClick((event) => {
    if (!clusterSelectionMode) return;
    
    const mouse = new THREE.Vector2();
    const rect = elem.getBoundingClientRect();
    
    mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
    
    const raycaster = new THREE.Raycaster();
    raycaster.setFromCamera(mouse, Graph.camera());
    
    const intersects = raycaster.intersectObjects(clusterMeshes);
    
    if (intersects.length > 0) {
        const clusterMesh = intersects[0].object;
        handleClusterClick(clusterMesh.userData.clusterId, clusterMesh.userData.nodes);
    }
});

// ---------------- Free Camera Controls ----------------
let move = { forward: false, backward: false, left: false, right: false };
let velocity = new THREE.Vector3();
let speed = 500;

function enableFreeCamera() {
    if (freeCameraEnabled) return;

    const camera = Graph.camera();
    freeCameraControls = new PointerLockControls(camera, document.body);

    const startFreeCamera = () => {
        freeCameraControls.lock();
        document.removeEventListener('click', startFreeCamera);
    };

    document.addEventListener('click', startFreeCamera);
    freeCameraEnabled = true;
    animateFreeCamera();
    console.log('Free camera mode enabled. Click to lock pointer.');
}

function animateFreeCamera() {
    if (!freeCameraEnabled) return;

    velocity.set(0, 0, 0);
    if (move.forward) velocity.z -= 1;
    if (move.backward) velocity.z += 1;
    if (move.left) velocity.x -= 1;
    if (move.right) velocity.x += 1;

    if (velocity.length() > 0) velocity.normalize();

    const delta = 0.5;
    velocity.multiplyScalar(speed * delta);

    freeCameraControls.moveRight(velocity.x);
    freeCameraControls.moveForward(velocity.z);

    requestAnimationFrame(animateFreeCamera);
}

// Key handlers for free camera
document.addEventListener('keydown', (event) => {
    if (!freeCameraEnabled) return;
    switch(event.code) {
        case 'ArrowUp': case 'KeyW': move.forward = true; break;
        case 'ArrowDown': case 'KeyS': move.backward = true; break;
        case 'ArrowLeft': case 'KeyA': move.left = true; break;
        case 'ArrowRight': case 'KeyD': move.right = true; break;
    }
});

document.addEventListener('keyup', (event) => {
    if (!freeCameraEnabled) return;
    switch(event.code) {
        case 'ArrowUp': case 'KeyW': move.forward = false; break;
        case 'ArrowDown': case 'KeyS': move.backward = false; break;
        case 'ArrowLeft': case 'KeyA': move.left = false; break;
        case 'ArrowRight': case 'KeyD': move.right = false; break;
    }
});

// ---------------- Lighting ----------------
Graph.scene().add(new THREE.AmbientLight(0x404040, 1.2));
const dirLight = new THREE.DirectionalLight(0xffffff, 1.5);
dirLight.position.set(200, 200, 200);
Graph.scene().add(dirLight);

const backLight = new THREE.DirectionalLight(0xffffff, 0.8);
backLight.position.set(-100, -100, -100);
Graph.scene().add(backLight);

// ---------------- Responsive ----------------
window.addEventListener('resize', () => {
    Graph.width(window.innerWidth);
    Graph.height(window.innerHeight);
});

// ---------------- Keyboard Controls ----------------
document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' || event.key === 'Backspace') {
        backToClusterSelection();
    } else if (event.key === 'c' || event.key === 'C') {
        toggleColorMode();
    } else if (event.key === 'f' || event.key === 'F') {
        toggleFrustumCulling();
    } else if (event.code === 'Space') {
        if (!freeCameraEnabled) {
            console.log("Entering free camera mode");
            enableFreeCamera();
        } else {
            console.log("Exiting free camera mode");
            freeCameraEnabled = false;
            if (freeCameraControls) freeCameraControls.unlock();
        }
    }
});

// ---------------- Start ----------------
animateCulling();
pollDotFiles('/semantic_graph.dot');

console.log('🎮 Controls:');
console.log('  ESC/Backspace - Return to cluster view');
console.log('  C - Toggle color mode (Cluster/Weight/Degree)');
console.log('  F - Toggle frustum culling');
console.log('  Space - Toggle free camera mode');