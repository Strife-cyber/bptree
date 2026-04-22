# 🌳 Db Engine Vis: B+ Tree & B-Tree Visualizer

> **Watch the intricate mechanics of deep, disk-backed Database B-Trees in real-time.**

Db Engine Vis is an incredibly powerful, interactive, and beautifully designed web visualizer for database internals. It features mathematically accurate **B+ Tree** and **B- Tree (Classical)** logic implemented in **Rust**, visually rendered directly onto a state-of-the-art **React** interface.

<div align="center">
  <!-- Add a screenshot here later if desired -->
</div>

---

## ✨ Features

- **Blazing Fast Rust Backend**: The core tree algorithms are implemented with extreme precision in Rust. Disk interactions simulate real page-level file mapping.
- **Side-by-Side Architectures**: Easily toggle between a pure Classical B-Tree and an industry-standard B+ Tree to understand the trade-offs of storing values at intermediate layers vs leaf nodes.
- **Deep Step-by-Step Transversal Animation**: Want to understand how a search works? Dial the speed back down to `0.1x` and watch the search traverse path down the tree, highlighting each evaluated key _within_ every node.
- **Configurable Tree Degree**: Need to observe splits more easily? Shrink the "Max Keys" per node down to `3`. Want to visualize production-like huge node fanouts? Increase the degree!
- **Batch Random Insert**: Press a single button to insert rapid batches of random data to observe cascade splits instantly.
- **Time Travel**: Drag the timeline slider back and forth to rewind history. Every insertion or deletion preserves the exact state of the tree structure at that specific moment.
- **Infinite Canvas Pan & Zoom**: Utilize a frictionless zoom & pan viewer, ensuring you can map massive node architectures regardless of scale or depth.

## 🚀 Getting Started

To run this application, you will need **Rust** and **Node.js** installed.

### 1. Start the Rust DB Engine

Navigate to the project root and start the backend HTTP server:
```bash
cargo run
```
This initializes `axum` on `http://127.0.0.1:3000` and creates the `.db` files representing our raw disk-backed B-Trees.

### 2. Start the React UI

In a new terminal window, navigate to the `ui` folder and start the Vite development server:
```bash
cd ui
npm install
npm run dev
```
Open `http://localhost:5173` in your browser.

## 🧠 Educational Insights

### Algorithm Complexities

Both B-Trees and B+ Trees offer robust logarithmic time complexities, making them the standard for disk-based storage where massive fan-outs flatten the depth of the tree.

| Operation | Average Case | Worst Case |
| :--- | :--- | :--- |
| **Search** | $\mathcal{O}(\log_t n)$ | $\mathcal{O}(\log_t n)$ |
| **Insert** | $\mathcal{O}(\log_t n)$ | $\mathcal{O}(\log_t n)$ |
| **Delete** | $\mathcal{O}(\log_t n)$ | $\mathcal{O}(\log_t n)$ |
| **Space** | $\mathcal{O}(n)$ | $\mathcal{O}(n)$ |

*(Where $t$ is the minimum degree of the tree, representing the massive horizontal fan-out per node).*

### The B+ Tree (Industry Standard)
In a **B+ Tree**, internal nodes store ONLY routing keys. All literal data values are physically pushed to the absolute bottom (the leaf nodes).
By stripping row values from the intermediate layers, nodes can pack exponentially more routing paths per 4KB page block. This extreme fan-out drastically minimizes disk I/O reads. This exact architecture powers massive production databases like PostgreSQL, MySQL (InnoDB), and SQLite.

### The Classical B-Tree
In a strictly **Classical B-Tree**, every single node persistently stores both its router key *and* its row data value. Notice how values propagate and reside through every depth level. While fetching a row at the root node is technically $O(1)$, the massive disk-page fragmentation throttles fan-out efficiency and cripples deeper sequential range scans.

## 🛠️ Tech Stack

- **Backend (Engine)**: Rust, Axum, Tokio, Bincode Serialization, File I/O.
- **Frontend (Visualizer)**: React (Vite), TypeScript, Framer Motion (for physics-based layout animations), React-Zoom-Pan-Pinch, Lucide Icons, Vanilla CSS.

## 🤝 Contributing

This project is built for learning! Feel free to modify the Rust backend to support custom tree implementations, or improve the React canvas. Pull requests are more than welcome.
