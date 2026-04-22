import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Database, HardDrive, Layers, ArrowRightLeft } from 'lucide-react';
import './index.css';

interface BTreeNode {
  id: number;
  type: 'internal' | 'leaf';
  keys: number[];
  values: string[];
  children?: BTreeNode[];
  next?: number | null;
}

const TreeNode = ({ node, treeType }: { node: BTreeNode, treeType: 'bplus' | 'bminus' }) => {
  return (
    <div className="btree-container">
      <motion.div
        layout
        initial={{ opacity: 0, y: -20, scale: 0.8 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, scale: 0.5 }}
        transition={{ type: 'spring', stiffness: 300, damping: 25 }}
        className={`node ${node.type}`}
      >
        <div className="node-id">Page {node.id}</div>
        {node.keys.map((key, i) => (
          <motion.div layout key={`${node.id}-${key}-${i}`} className="key-cell">
            <span>{key}</span>
            {((treeType === 'bplus' && node.type === 'leaf') || treeType === 'bminus') && node.values && node.values[i] && (
              <span className="key-value">{node.values[i]}</span>
            )}
          </motion.div>
        ))}
        {treeType === 'bplus' && node.type === 'leaf' && node.next !== null && node.next !== undefined && (
          <div className="key-cell bg-accent">
            <span style={{ fontSize: '0.6rem' }}>NEXT: {node.next}</span>
          </div>
        )}
      </motion.div>
      
      {node.children && node.children.length > 0 && (
        <div className="node-row">
          <AnimatePresence>
            {node.children.map((child) => (
              <TreeNode key={child.id} node={child} treeType={treeType} />
            ))}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
};

export default function App() {
  const [treeType, setTreeType] = useState<'bplus' | 'bminus'>('bplus');
  const [tree, setTree] = useState<BTreeNode | null>(null);
  const [insertKey, setInsertKey] = useState('');
  const [insertValue, setInsertValue] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    fetchTree();
  }, [treeType]);

  const fetchTree = async () => {
    try {
      const endpoint = treeType === 'bplus' ? 'tree' : 'btree';
      const res = await fetch(`http://127.0.0.1:3000/${endpoint}`);
      const data = await res.json();
      setTree(data);
    } catch (e) {
      console.error('Failed to fetch tree', e);
    }
  };

  const handleInsert = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!insertKey) return;
    
    setLoading(true);
    try {
      const endpoint = treeType === 'bplus' ? 'insert' : 'insert_btree';
      const res = await fetch(`http://127.0.0.1:3000/${endpoint}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          key: parseInt(insertKey),
          value: insertValue || `Val-${insertKey}`
        })
      });
      const data = await res.json();
      setTree(data);
      setInsertKey('');
      setInsertValue('');
    } catch (e) {
      console.error('Failed to insert', e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app-container">
      <div className="sidebar">
        <div>
          <h1 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.5rem' }}>
            <Database size={24} color="var(--accent-blue)" />
            Db Engine Visualizer
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
            Visualize Disk-Backed B-Trees and B+ Trees in real-time.
          </p>
        </div>

        <div style={{ display: 'flex', gap: '1rem', background: 'rgba(255,255,255,0.05)', padding: '0.5rem', borderRadius: '8px' }}>
          <button 
            type="button"
            onClick={() => setTreeType('bplus')}
            style={{ 
              flex: 1, 
              background: treeType === 'bplus' ? 'linear-gradient(135deg, var(--accent-blue), var(--accent-purple))' : 'transparent',
              padding: '0.5rem'
            }}
          >
            B+ Tree
          </button>
          <button 
            type="button"
            onClick={() => setTreeType('bminus')}
            style={{ 
              flex: 1, 
              background: treeType === 'bminus' ? 'linear-gradient(135deg, var(--accent-blue), var(--accent-purple))' : 'transparent',
              padding: '0.5rem'
            }}
          >
            B-Tree
          </button>
        </div>

        <form onSubmit={handleInsert} className="controls">
          <div className="input-group">
            <input
              type="number"
              placeholder="Key (e.g., 42)"
              value={insertKey}
              onChange={(e) => setInsertKey(e.target.value)}
              required
            />
            <input
              type="text"
              placeholder="Value"
              value={insertValue}
              onChange={(e) => setInsertValue(e.target.value)}
            />
          </div>
          <button type="submit" disabled={loading}>
            {loading ? 'Inserting...' : `Insert into ${treeType === 'bplus' ? 'B+ Tree' : 'B-Tree'}`}
          </button>
        </form>

        <div className="theory-panel">
          {treeType === 'bplus' ? (
            <>
              <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <Layers size={18} />
                B+ Tree Advantage
              </h2>
              <p>
                In a <b>B+ Tree</b>, internal nodes store ONLY routing keys, and all values are pushed into the leaf nodes. 
              </p>
              <p>
                Notice how the values (Val-XYZ) only appear at the bottom level. Because internal nodes don't waste space on values, their "fan-out" is much higher, resulting in a shallower tree!
              </p>
            </>
          ) : (
            <>
              <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <ArrowRightLeft size={18} />
                Standard B-Tree Mechanism
              </h2>
              <p>
                In a <b>Standard B-Tree</b>, every node stores both the key and the row data value. 
              </p>
              <p>
                Notice how values are stored at every depth, including the root node. While this can make retrieving keys that exist near the root slightly faster, it massively lowers the network fan-out and leads to deeper disk seeks for the majority of queries.
              </p>
            </>
          )}

          <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: '1.5rem' }}>
            <HardDrive size={18} />
            Mechanical Sympathy
          </h2>
          <p>
            Both engines write perfectly aligned pages directly to a local binary file. When you insert items, watch the nodes "overflow" and instantly split!
          </p>
        </div>
      </div>

      <div className="visualizer-area">
        {tree && tree.keys ? (
          <TreeNode node={tree} treeType={treeType} />
        ) : (
          <div style={{ color: 'var(--text-secondary)' }}>Loading Engine State...</div>
        )}
      </div>
    </div>
  );
}
