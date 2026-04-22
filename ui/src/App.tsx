import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Database, HardDrive, Layers, ArrowRightLeft, Plus, Rewind, FastForward } from 'lucide-react';
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
    <li>
      <a href="#">
        <motion.div
           layout
           initial={{ opacity: 0, scale: 0.8, y: -20 }}
           animate={{ opacity: 1, scale: 1, y: 0 }}
           transition={{ type: 'spring', stiffness: 350, damping: 25 }}
           className={`node-box ${node.type}`}
        >
          <div className="node-id">Page {node.id}</div>
          {node.keys.map((key, i) => (
            <motion.div layout key={`${node.id}-${key}`} className="kv-pair">
              <div className="k-top">{key}</div>
              {((treeType === 'bplus' && node.type === 'leaf') || treeType === 'bminus') && node.values && node.values[i] && (
                <div className="v-bottom">{node.values[i]}</div>
              )}
            </motion.div>
          ))}
          {treeType === 'bplus' && node.type === 'leaf' && node.next !== null && node.next !== undefined && (
             <div className="next-ptr">→ P{node.next}</div>
          )}
        </motion.div>
      </a>
      
      {node.children && node.children.length > 0 && (
        <ul>
          <AnimatePresence>
            {node.children.map((child) => (
              <TreeNode key={child.id} node={child} treeType={treeType} />
            ))}
          </AnimatePresence>
        </ul>
      )}
    </li>
  );
};

export default function App() {
  const [treeType, setTreeType] = useState<'bplus' | 'bminus'>('bplus');
  
  // History state
  const [historyPlus, setHistoryPlus] = useState<BTreeNode[]>([]);
  const [historyMinus, setHistoryMinus] = useState<BTreeNode[]>([]);
  const [indexPlus, setIndexPlus] = useState(-1);
  const [indexMinus, setIndexMinus] = useState(-1);

  const [insertKey, setInsertKey] = useState('');
  const [insertValue, setInsertValue] = useState('');
  const [loading, setLoading] = useState(false);

  // Initialize
  useEffect(() => {
    fetchTree('bplus');
    fetchTree('bminus');
  }, []);

  const fetchTree = async (type: 'bplus' | 'bminus') => {
    try {
      const endpoint = type === 'bplus' ? 'tree' : 'btree';
      const res = await fetch(`http://127.0.0.1:3000/${endpoint}`);
      const data = await res.json();
      
      if (type === 'bplus') {
         setHistoryPlus([data]);
         setIndexPlus(0);
      } else {
         setHistoryMinus([data]);
         setIndexMinus(0);
      }
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
      // First hit insert endpoint
      await fetch(`http://127.0.0.1:3000/${endpoint}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          key: parseInt(insertKey),
          value: insertValue || `Val-${insertKey}`
        })
      });

      // Need to fetch fresh tree state to display (insert endpoint sometimes echoes state, but let's be safe)
      const fetchEndpoint = treeType === 'bplus' ? 'tree' : 'btree';
      const res = await fetch(`http://127.0.0.1:3000/${fetchEndpoint}`);
      const data = await res.json();
      
      if (treeType === 'bplus') {
         const newHistory = historyPlus.slice(0, indexPlus + 1);
         setHistoryPlus([...newHistory, data]);
         setIndexPlus(newHistory.length);
      } else {
         const newHistory = historyMinus.slice(0, indexMinus + 1);
         setHistoryMinus([...newHistory, data]);
         setIndexMinus(newHistory.length);
      }
      
      setInsertKey('');
      setInsertValue('');
    } catch (e) {
      console.error('Failed to insert', e);
    } finally {
      setLoading(false);
    }
  };
  
  const history = treeType === 'bplus' ? historyPlus : historyMinus;
  const currentIndex = treeType === 'bplus' ? indexPlus : indexMinus;
  const currentTree = history[currentIndex] || null;

  const handleSliderChange = (e: any) => {
     const idx = parseInt(e.target.value);
     if (treeType === 'bplus') setIndexPlus(idx);
     else setIndexMinus(idx);
  };

  return (
    <div className="app-container">
      <div className="sidebar">
        <div className="title-header">
          <h1>
            <Database size={28} color="var(--accent-base)" />
            Db Engine Vis
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem', marginTop: '0.8rem', lineHeight: '1.5' }}>
            Watch the intricate mechanics of deep disk-backed B-Trees in real time. We built mathematically accurate Rust storage logic that renders precisely onto the React layer.
          </p>
        </div>

        <div className="controls-card">
          <div className="toggle-group">
            <button 
              type="button"
              className={`toggle-btn ${treeType === 'bplus' ? 'active' : ''}`}
              onClick={() => { setTreeType('bplus'); }}
            >
              B+ Tree
            </button>
            <button 
              type="button"
              className={`toggle-btn ${treeType === 'bminus' ? 'active' : ''}`}
              onClick={() => { setTreeType('bminus'); }}
            >
              Standard B-Tree
            </button>
          </div>

          <form onSubmit={handleInsert}>
            <div className="input-group">
              <input
                type="number"
                placeholder="Key: e.g., 42"
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
            <button type="submit" disabled={loading} className="insert-btn">
              <Plus size={18} />
              {loading ? 'Committing...' : `Insert Record`}
            </button>
          </form>

          {history.length > 0 && (
            <div className="history-controls">
              <div className="history-header">
                 <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <Rewind size={14}/> Time Travel Array
                 </span>
                 <span className="step-badge">State {currentIndex} / {history.length - 1}</span>
              </div>
              <input 
                type="range" 
                min="0" 
                max={history.length > 0 ? history.length - 1 : 0} 
                value={currentIndex} 
                onChange={handleSliderChange}
                className="slider"
              />
            </div>
          )}
        </div>

        <div className="controls-card">
          {treeType === 'bplus' ? (
            <>
              <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '1.1rem', marginBottom: '1rem', color: '#f8fafc' }}>
                <Layers size={20} color="var(--accent-glow)" />
                B+ Tree Architecture
              </h2>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: '1.6' }}>
                In a <b>B+ Tree</b>, internal nodes store ONLY routing keys, and all values are physically pushed into the leaf nodes. 
              </p>
              <br/>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: '1.6' }}>
                By stripping row values from intermediate layers, nodes can pack exponentially more routing paths per 4KB page block. This extreme fan-out is exactly why production systems like PostgreSQL use them!
              </p>
            </>
          ) : (
            <>
              <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '1.1rem', marginBottom: '1rem', color: '#f8fafc' }}>
                <ArrowRightLeft size={20} color="var(--accent-success)" />
                Classical B-Tree
              </h2>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: '1.6' }}>
                In a strictly <b>Classical B-Tree</b>, every single node persistently stores both its router key and its row data value.
              </p>
              <br/>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: '1.6' }}>
                Notice how values propagate through every depth. While retrieving the root node is technically O(1), the massive disk-page fragmentation throttles fan-out efficiency and cripples deeper sequential range scans.
              </p>
            </>
          )}
        </div>
      </div>

      <div className="visualizer-area">
        <div className="canvas-container">
          <div className="tree">
            {currentTree ? (
              <ul>
                <TreeNode node={currentTree} treeType={treeType} />
              </ul>
            ) : (
              <div style={{ color: 'var(--text-secondary)', marginTop: '4rem' }}>Awaiting Disk Initialization...</div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
