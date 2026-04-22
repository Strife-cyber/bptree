import { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { TransformWrapper, TransformComponent } from 'react-zoom-pan-pinch';
import {
  Database, Layers, ArrowRightLeft, Plus, Rewind,
  Search, Trash2, RefreshCw, ZoomIn, ZoomOut, Maximize, Pause,
  Settings2, Target, X
} from 'lucide-react';
import './index.css';

interface BTreeNode {
  id: number;
  type: 'internal' | 'leaf';
  keys: number[];
  values: string[];
  children?: BTreeNode[];
  next?: number | null;
}

interface TreeNodeProps {
  node: BTreeNode;
  treeType: 'bplus' | 'bminus';
  highlightedNodes: Set<number>;
  searchTargetKey: { nodeId: number, keyIndex: number } | null;
  searchStep: number;
}

const TreeNode = ({ node, treeType, highlightedNodes, searchTargetKey, searchStep }: TreeNodeProps) => {
  const isHighlighted = highlightedNodes.has(node.id);
  const isCurrentSearch = searchStep === node.id;

  return (
    <li>
      <a href="#" onClick={(e) => e.preventDefault()}>
        <motion.div
           layout
           initial={{ opacity: 0, scale: 0.8, y: -20 }}
           animate={{
             opacity: 1,
             scale: isHighlighted ? 1.05 : 1,
             y: 0,
             boxShadow: isCurrentSearch
               ? '0 0 30px rgba(245, 158, 11, 0.6)'
               : isHighlighted
                 ? '0 0 20px rgba(99, 102, 241, 0.4)'
                 : '0 10px 30px -10px rgba(0,0,0,0.8)'
           }}
           transition={{ type: 'spring', stiffness: 350, damping: 25 }}
           className={`node-box ${node.type} ${isHighlighted ? 'highlighted' : ''} ${isCurrentSearch ? 'search-current' : ''}`}
        >
          <div className="node-id">Page {node.id}</div>
          {node.keys.map((key, i) => {
            const isComparing = searchTargetKey?.nodeId === node.id && searchTargetKey.keyIndex === i;
            return (
            <motion.div
              layout
              key={`${node.id}-${key}`}
              className={`kv-pair ${isComparing ? 'search-target' : ''}`}
            >
              <div className="k-top">{key}</div>
              {((treeType === 'bplus' && node.type === 'leaf') || treeType === 'bminus') && node.values && node.values[i] && (
                <div className="v-bottom">{node.values[i]}</div>
              )}
            </motion.div>
          )})}
          {treeType === 'bplus' && node.type === 'leaf' && node.next !== null && node.next !== undefined && (
             <div className="next-ptr">→ P{node.next}</div>
          )}
        </motion.div>
      </a>

      {node.children && node.children.length > 0 && (
        <ul>
          <AnimatePresence>
            {node.children.map((child) => (
              <TreeNode
                key={child.id}
                node={child}
                treeType={treeType}
                highlightedNodes={highlightedNodes}
                searchTargetKey={searchTargetKey}
                searchStep={searchStep}
              />
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
  const [searchKey, setSearchKey] = useState('');
  const [deleteKey, setDeleteKey] = useState('');
  const [updateKey, setUpdateKey] = useState('');
  const [updateValue, setUpdateValue] = useState('');
  const [loading, setLoading] = useState(false);

  const [highlightedNodes, setHighlightedNodes] = useState<Set<number>>(new Set());
  const [searchTargetKey, setSearchTargetKey] = useState<{ nodeId: number, keyIndex: number } | null>(null);
  const [searchCurrentStep, setSearchCurrentStep] = useState<number>(-1);
  const [isSearching, setIsSearching] = useState(false);
  const [searchSpeed, setSearchSpeed] = useState(1000);
  const [isPaused, setIsPaused] = useState(false);
  
  const isSearchingRef = useRef(false);
  const isPausedRef = useRef(false);
  const speedRef = useRef(1000);
  
  // Sync speed to ref
  useEffect(() => {
    speedRef.current = searchSpeed;
  }, [searchSpeed]);
  
  const [treeDegree, setTreeDegree] = useState(3);

  const transformRef = useRef<any>(null);

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

  const handleBatchInsert = async () => {
    setLoading(true);
    try {
      const endpoint = treeType === 'bplus' ? 'insert' : 'insert_btree';
      for(let i=0; i<10; i++) {
        const randomKey = Math.floor(Math.random() * 1000);
        await fetch(`http://127.0.0.1:3000/${endpoint}`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ key: randomKey, value: `Val-${randomKey}` })
        });
      }
      
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
    } catch (e) {
      console.error('Failed batch insert', e);
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchKey || isSearching) return;

    const key = parseInt(searchKey);
    setIsSearching(true);
    setIsPaused(false);
    isSearchingRef.current = true;
    isPausedRef.current = false;
    
    setSearchTargetKey(null);
    setHighlightedNodes(new Set());
    setSearchCurrentStep(-1);

    try {
      const endpoint = treeType === 'bplus' ? 'search' : 'search_btree';
      const res = await fetch(`http://127.0.0.1:3000/${endpoint}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key })
      });
      const data = await res.json();
      const path: Array<{id: number, found: boolean, keys: number[]}> = data.path;

      for (let i = 0; i < path.length; i++) {
        if (!isSearchingRef.current) break;
        while (isPausedRef.current && isSearchingRef.current) {
          await new Promise(r => setTimeout(r, 100));
        }
        
        const node = path[i];
        setSearchCurrentStep(node.id);
        setHighlightedNodes(prev => new Set([...prev, node.id]));
        await new Promise(r => setTimeout(r, speedRef.current));
        
        let matched = false;
        for (let j = 0; j < node.keys.length; j++) {
           if (!isSearchingRef.current) break;
           while (isPausedRef.current && isSearchingRef.current) {
             await new Promise(r => setTimeout(r, 100));
           }
           
           setSearchTargetKey({ nodeId: node.id, keyIndex: j });
           await new Promise(r => setTimeout(r, speedRef.current));
           
           if (key === node.keys[j]) {
              matched = true;
              break;
           }
           if (key < node.keys[j]) {
              break;
           }
        }
        
        if (matched && node.found) {
            // Found it permanently
            break;
        } else {
            setSearchTargetKey(null);
        }
        
        if (i < path.length - 1) {
           await new Promise(r => setTimeout(r, speedRef.current));
        }
      }

      await new Promise(r => setTimeout(r, speedRef.current));
    } catch (e) {
      console.error('Failed to search', e);
    } finally {
      setIsSearching(false);
      isSearchingRef.current = false;
      setSearchCurrentStep(-1);
    }
  };

  const cancelSearch = () => {
    setIsSearching(false);
    setIsPaused(false);
    isSearchingRef.current = false;
    isPausedRef.current = false;
    setSearchCurrentStep(-1);
    setHighlightedNodes(new Set());
    setSearchTargetKey(null);
  };

  const togglePause = () => {
     setIsPaused(p => {
        isPausedRef.current = !p;
        return !p;
     });
  };

  const handleDelete = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!deleteKey) return;

    setLoading(true);
    try {
      const endpoint = treeType === 'bplus' ? 'delete' : 'delete_btree';
      await fetch(`http://127.0.0.1:3000/${endpoint}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key: parseInt(deleteKey) })
      });

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

      setDeleteKey('');
    } catch (e) {
      console.error('Failed to delete', e);
    } finally {
      setLoading(false);
    }
  };

  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!updateKey || !updateValue) return;

    setLoading(true);
    try {
      const endpoint = treeType === 'bplus' ? 'insert' : 'insert_btree';
      await fetch(`http://127.0.0.1:3000/${endpoint}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          key: parseInt(updateKey),
          value: updateValue
        })
      });

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

      setUpdateKey('');
      setUpdateValue('');
    } catch (e) {
      console.error('Failed to update', e);
    } finally {
      setLoading(false);
    }
  };

  const handleReset = async () => {
    setLoading(true);
    try {
      await fetch(`http://127.0.0.1:3000/reset`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ max_keys: treeDegree || 3 })
      });
      setHistoryPlus([]);
      setHistoryMinus([]);
      setIndexPlus(-1);
      setIndexMinus(-1);
      await fetchTree('bplus');
      await fetchTree('bminus');
    } catch (e) {
      console.error('Failed to reset', e);
    } finally {
      setLoading(false);
    }
  };

  const resetView = () => {
    if (transformRef.current) {
      transformRef.current.resetTransform();
    }
  };

  const zoomIn = () => {
    if (transformRef.current) {
      transformRef.current.zoomIn();
    }
  };

  const zoomOut = () => {
    if (transformRef.current) {
      transformRef.current.zoomOut();
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
            <button type="button" className={`toggle-btn ${treeType === 'bplus' ? 'active' : ''}`} onClick={() => setTreeType('bplus')}>
              B+ Tree
            </button>
            <button type="button" className={`toggle-btn ${treeType === 'bminus' ? 'active' : ''}`} onClick={() => setTreeType('bminus')}>
              Standard B-Tree
            </button>
          </div>

          <form onSubmit={handleSearch} className="search-form">
            <div className="section-header">
              <Search size={16} color="var(--accent-warning)" />
              <span>Search Animation</span>
            </div>
            <div className="input-group">
              <input type="number" placeholder="Search key..." value={searchKey} onChange={(e) => setSearchKey(e.target.value)} disabled={isSearching} />
              {isSearching ? (
                <>
                  <button type="button" onClick={cancelSearch} className={"icon-btn danger"} title="Cancel Search"><X size={18} /></button>
                  <button type="button" onClick={togglePause} className={"icon-btn"} style={{ background: 'rgba(255,255,255,0.1)' }} title={isPaused ? "Resume" : "Pause"}>
                     {isPaused ? <Target size={18} /> : <Pause size={18} />}
                  </button>
                </>
              ) : (
                <button type="submit" disabled={!searchKey || isSearching} className={"icon-btn primary"}><Search size={18} /></button>
              )}
            </div>
            <div className="speed-control">
              <span className="speed-label"><Settings2 size={12} /> Speed</span>
              <input type="range" min="100" max="10000" step="100" value={10100 - searchSpeed} onChange={(e) => setSearchSpeed(10100 - parseInt(e.target.value))} disabled={isSearching} className="speed-slider" />
              <span className="speed-value">{(1000 / searchSpeed).toFixed(1)}x</span>
            </div>
            {isSearching && (
              <div className="search-status">
                {isPaused ? <span className={"status paused"}><Pause size={12} /> Paused</span> : <span className={"status searching"}><Target size={12} /> Searching...</span>}
              </div>
            )}
          </form>

          <form onSubmit={handleInsert} className="action-form">
            <div className="section-header"><Plus size={16} color="var(--accent-success)" /><span>Insert Record</span></div>
            <div className="input-group">
              <input type="number" placeholder="Key: e.g., 42" value={insertKey} onChange={(e) => setInsertKey(e.target.value)} disabled={loading} />
              <input type="text" placeholder="Value" value={insertValue} onChange={(e) => setInsertValue(e.target.value)} disabled={loading} />
            </div>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <button type="submit" disabled={loading || !insertKey} className={"action-btn insert"} style={{ flex: 2 }}>{loading ? 'Committing...' : 'Insert'}</button>
              <button type="button" onClick={handleBatchInsert} disabled={loading} className={"action-btn"} style={{ flex: 1, background: 'rgba(255,255,255,0.1)', color: 'white' }}>Batch Random</button>
            </div>
          </form>

          <form onSubmit={handleUpdate} className="action-form">
            <div className="section-header"><RefreshCw size={16} color="var(--accent-glow)" /><span>Update Value</span></div>
            <div className="input-group">
              <input type="number" placeholder="Existing key..." value={updateKey} onChange={(e) => setUpdateKey(e.target.value)} disabled={loading} />
              <input type="text" placeholder="New value..." value={updateValue} onChange={(e) => setUpdateValue(e.target.value)} disabled={loading} />
            </div>
            <button type="submit" disabled={loading || !updateKey || !updateValue} className={"action-btn update"}>{loading ? 'Updating...' : 'Update'}</button>
          </form>

          <form onSubmit={handleDelete} className="action-form">
            <div className="section-header"><Trash2 size={16} color="#ef4444" /><span>Delete Key</span></div>
            <div className="input-group">
              <input type="number" placeholder="Key to delete..." value={deleteKey} onChange={(e) => setDeleteKey(e.target.value)} disabled={loading} />
              <button type="submit" disabled={loading || !deleteKey} className={"icon-btn danger"}><Trash2 size={18} /></button>
            </div>
          </form>

          {history.length > 0 && (
            <div className="history-controls">
              <div className="history-header">
                <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}><Rewind size={14}/> Time Travel Array</span>
                <span className="step-badge">State {currentIndex} / {history.length - 1}</span>
              </div>
              <input type="range" min="0" max={history.length > 0 ? history.length - 1 : 0} value={currentIndex} onChange={handleSliderChange} className="slider" />
            </div>
          )}

          <div className="action-form" style={{ marginTop: '1.5rem', borderBottom: 'none' }}>
             <div className="section-header"><Settings2 size={16} color="var(--text-secondary)" /><span>Tree Configuration</span></div>
             <div className="input-group">
                <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', alignSelf: 'center', whiteSpace: 'nowrap' }}>Max Keys:</span>
                <input type="number" min="3" max="20" value={treeDegree} onChange={(e) => setTreeDegree(parseInt(e.target.value))} disabled={loading} />
             </div>
             <button type="button" onClick={handleReset} disabled={loading} className={"action-btn danger"} style={{ background: 'rgba(239, 68, 68, 0.2)', color: '#ef4444', border: '1px solid rgba(239, 68, 68, 0.3)' }}>Reset Trees (Wipe Data)</button>
          </div>
        </div>

        <div className="controls-card">
          {treeType === 'bplus' ? (
            <>
              <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '1.1rem', marginBottom: '1rem', color: '#f8fafc' }}>
                <Layers size={20} color="var(--accent-glow)" /> B+ Tree Architecture
              </h2>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: '1.6' }}>In a <b>B+ Tree</b>, internal nodes store ONLY routing keys, and all values are physically pushed into the leaf nodes.</p>
              <br/>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: '1.6' }}>By stripping row values from intermediate layers, nodes can pack exponentially more routing paths per 4KB page block. This extreme fan-out is exactly why production systems like PostgreSQL use them!</p>
            </>
          ) : (
            <>
              <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '1.1rem', marginBottom: '1rem', color: '#f8fafc' }}>
                <ArrowRightLeft size={20} color="var(--accent-success)" /> Classical B-Tree
              </h2>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: '1.6' }}>In a strictly <b>Classical B-Tree</b>, every single node persistently stores both its router key and its row data value.</p>
              <br/>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: '1.6' }}>Notice how values propagate through every depth. While retrieving the root node is technically O(1), the massive disk-page fragmentation throttles fan-out efficiency and cripples deeper sequential range scans.</p>
            </>
          )}
        </div>
      </div>

      <div className="visualizer-area">
        <div className="zoom-controls">
          <button onClick={zoomIn} className="zoom-btn" title="Zoom In"><ZoomIn size={20} /></button>
          <button onClick={zoomOut} className="zoom-btn" title="Zoom Out"><ZoomOut size={20} /></button>
          <button onClick={resetView} className="zoom-btn" title="Fit to Screen"><Maximize size={20} /></button>
        </div>

        <TransformWrapper ref={transformRef} initialScale={1} minScale={0.05} maxScale={3} centerOnInit={true} limitToBounds={false}>
          {() => (
            <TransformComponent wrapperClass="transform-wrapper" contentClass="transform-content">
              <div className="canvas-container">
                <div className="tree">
                  {currentTree ? (
                    <ul>
                      <TreeNode node={currentTree} treeType={treeType} highlightedNodes={highlightedNodes} searchTargetKey={searchTargetKey} searchStep={searchCurrentStep} />
                    </ul>
                  ) : (
                    <div style={{ color: 'var(--text-secondary)', marginTop: '4rem' }}>Awaiting Disk Initialization...</div>
                  )}
                </div>
              </div>
            </TransformComponent>
          )}
        </TransformWrapper>
      </div>
    </div>
  );
}
