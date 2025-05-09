import { useState, useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Link, useNavigate, useLocation } from 'react-router-dom';
import logoImage from './logo.jpg';
import './App.css';

const styles = {
  container: {
    maxWidth: '1000px',
    margin: '0 auto',
    padding: '20px',
    fontFamily: 'Arial, sans-serif',
    minHeight: '100vh'
  },
  centerContainer: {
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
    marginTop: '50px'
  },
  logoContainer: {
    marginBottom: '30px',
    textAlign: 'center' as const
  },
  logoPlaceholder: {
    width: '250px',
    height: '100px',
    backgroundColor: '#f0f0f0',
    margin: '0 auto 20px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: '8px'
  },
  logoText: {
    fontSize: '48px',
    fontWeight: 'bold'
  },
  logoR: { color: '#4285F4' },
  logoU: { color: '#EA4335' },
  logoS: { color: '#FBBC05' },
  logoT: { color: '#4285F4' },
  logoL: { color: '#34A853' },
  logoE: { color: '#EA4335' },
  searchContainer: {
    width: '100%',
    maxWidth: '600px',
    marginBottom: '20px'
  },
  searchForm: {
    display: 'flex',
    gap: '10px',
    marginBottom: '10px'
  },
  searchInput: {
    flex: '1',
    padding: '12px 20px',
    fontSize: '16px',
    border: '1px solid #dfe1e5',
    borderRadius: '24px',
    boxShadow: '0 1px 6px rgba(32, 33, 36, 0.28)',
    outline: 'none'
  },
  searchButton: {
    padding: '0 20px',
    backgroundColor: '#4285F4',
    color: 'white',
    border: 'none',
    borderRadius: '24px',
    fontSize: '16px',
    fontWeight: 'bold',
    cursor: 'pointer'
  },
  disabledButton: {
    backgroundColor: '#ccc',
    cursor: 'not-allowed'
  },
  searchStats: {
    display: 'flex',
    justifyContent: 'space-between',
    fontSize: '12px',
    color: '#666',
    padding: '0 20px'
  },
  resultsContainer: {
    marginTop: '20px'
  },
  searchTime: {
    fontSize: '12px',
    color: '#666',
    borderBottom: '1px solid #dfe1e5',
    paddingBottom: '10px',
    marginBottom: '20px'
  },
  loadingContainer: {
    textAlign: 'center' as const,
    padding: '40px'
  },
  loader: {
    border: '4px solid #f3f3f3',
    borderTop: '4px solid #3498db',
    borderRadius: '50%',
    width: '30px',
    height: '30px',
    animation: 'spin 2s linear infinite',
    margin: '0 auto 10px'
  },
  resultItem: {
    marginBottom: '25px',
    padding: '0 15px',
    cursor: 'pointer'
  },
  resultTitle: {
    fontSize: '18px',
    fontWeight: '500',
    color: '#1a0dab',
    marginBottom: '3px',
    textDecoration: 'none'
  },
  resultMeta: {
    fontSize: '12px',
    color: '#006621',
    marginBottom: '3px'
  },
  resultAuthor: {
    fontSize: '14px',
    color: '#666',
    marginBottom: '3px'
  },
  resultSnippet: {
    fontSize: '14px',
    color: '#333',
    lineHeight: '1.5'
  },
  documentContainer: {
    maxWidth: '800px',
    margin: '0 auto',
    padding: '20px',
    fontFamily: 'Arial, sans-serif'
  },
  documentHeader: {
    display: 'flex',
    alignItems: 'center',
    marginBottom: '30px'
  },
  backButton: {
    marginRight: '15px',
    padding: '10px',
    backgroundColor: 'transparent',
    border: 'none',
    cursor: 'pointer',
    borderRadius: '50%'
  },
  documentContent: {
    backgroundColor: 'white',
    padding: '20px',
    borderRadius: '8px',
    boxShadow: '0 1px 3px rgba(0,0,0,0.12)'
  },
  documentTitle: {
    fontSize: '24px',
    fontWeight: 'bold',
    marginBottom: '15px',
    paddingBottom: '10px',
    borderBottom: '1px solid #dfe1e5'
  },
  documentSection: {
    marginBottom: '20px'
  },
  documentSectionTitle: {
    fontSize: '18px',
    fontWeight: '500',
    marginBottom: '10px'
  },
  documentText: {
    backgroundColor: '#f8f9fa',
    padding: '15px',
    borderRadius: '4px',
    whiteSpace: 'pre-line' as const,
    lineHeight: '1.5'
  },
  documentFooter: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    backgroundColor: '#f8f9fa',
    padding: '10px',
    borderRadius: '4px',
    marginTop: '20px'
  },
  documentId: {
    fontSize: '14px',
    color: '#666'
  },
  documentScore: {
    fontSize: '12px',
    backgroundColor: '#e8f0fe',
    color: '#1a73e8',
    padding: '3px 8px',
    borderRadius: '12px'
  },
  errorContainer: {
    padding: '15px',
    backgroundColor: '#fff8f8',
    border: '1px solid #fcc',
    color: '#d33',
    borderRadius: '4px'
  }
};

interface Document {
  id: string;
  title: string;
  text: string;
  score: number;
}

interface Stats {
  document_count: number;
  vocabulary_size: number;
}

function SearchPage() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<Document[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [apiStatus, setApiStatus] = useState<string>('checking');
  const [stats, setStats] = useState<Stats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [resultCount, setResultCount] = useState<number>(10);
  const [searchTime, setSearchTime] = useState<number | null>(null);
  const navigate = useNavigate();

  const API_URL = 'http://127.0.0.1:8080';

  useEffect(() => {
    checkApiConnection();
  }, []);

  const checkApiConnection = async () => {
    try {
      setApiStatus('checking');
      
      const response = await fetch(`${API_URL}/stats`, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      });
      
      if (response.ok) {
        const statsData = await response.json();
        setStats(statsData);
        setApiStatus('connected');
      } else {
        setApiStatus('error');
        setError('API responded with an error');
      }
    } catch (error) {
      console.error('API connection error:', error);
      setApiStatus('error');
      setError('Cannot connect to API server. Is it running at http://127.0.0.1:8080?');
    }
  };

  const handleSearch = async () => {
    if (!query.trim()) return;
    
    setLoading(true);
    setResults([]);
    setError(null);
    
    const startTime = performance.now();
    
    try {
      const response = await fetch(`${API_URL}/search`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ 
          query: query,
          limit: resultCount
        }),
      });
      
      if (!response.ok) {
        throw new Error(`API responded with status: ${response.status}`);
      }
      
      const data = await response.json();
      
      const endTime = performance.now();
      setSearchTime((endTime - startTime) / 1000); // Convert to seconds
      
      setResults(data);
      
      if (data.length === 0) {
        setError(`No results found for "${query}"`);
      }
    } catch (error) {
      console.error('Error searching:', error);
      setError(`Search failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
      setSearchTime(null);
    } finally {
      setLoading(false);
    }
  };

  const handleViewDocument = (docId: string) => {
    navigate(`/document/${docId}`, { state: { document: results.find(doc => doc.id === docId) } });
  };

  return (
    <div style={styles.container}>
      {}
      <div style={styles.centerContainer}>
        {}
        <div style={styles.logoContainer}>
          <img 
            src={logoImage} 
            alt="Logo" 
            style={{ 
              width: '250px', 
              height: 'auto',
              marginBottom: '20px'
            }} 
          />
        </div>

        {}
        <div style={styles.searchContainer}>
          {}
          {apiStatus === 'error' && (
            <div style={styles.errorContainer}>
              <div>
                <span>⚠️ API Connection Failed</span>
              </div>
              <div>
                <p>{error}</p>
                <button 
                  onClick={checkApiConnection}
                  style={{
                    marginTop: '10px',
                    padding: '5px 10px',
                    backgroundColor: 'white',
                    border: '1px solid #fcc', 
                    borderRadius: '4px',
                    color: '#d33',
                    cursor: 'pointer'
                  }}
                >
                  Retry Connection
                </button>
              </div>
            </div>
          )}

          <div style={styles.searchForm}>
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search documents..."
              style={styles.searchInput}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleSearch();
                }
              }}
              disabled={apiStatus !== 'connected'}
            />
            
            <button
              onClick={handleSearch}
              disabled={loading || apiStatus !== 'connected'}
              style={{
                ...styles.searchButton,
                ...(apiStatus !== 'connected' ? styles.disabledButton : {})
              }}
            >
              {loading ? 'Searching...' : 'Search'}
            </button>
          </div>
          
          <div style={styles.searchStats}>
            <div>
              {stats && apiStatus === 'connected' && (
                <span>Indexed documents: {stats.document_count} | Vocabulary: {stats.vocabulary_size}</span>
              )}
            </div>
            <div>
              <label htmlFor="resultCount" style={{marginRight: '5px'}}>Results:</label>
              <select
                id="resultCount"
                value={resultCount}
                onChange={(e) => setResultCount(Number(e.target.value))}
                style={{
                  border: '1px solid #dfe1e5', 
                  borderRadius: '4px', 
                  padding: '2px'
                }}
                disabled={apiStatus !== 'connected'}
              >
                <option value={5}>5</option>
                <option value={10}>10</option>
                <option value={25}>25</option>
                <option value={50}>50</option>
              </select>
            </div>
          </div>
        </div>
      </div>

      {}
      {(loading || results.length > 0 || error) && (
        <div style={styles.resultsContainer}>
          {}
          {searchTime !== null && results.length > 0 && (
            <div style={styles.searchTime}>
              About {results.length} results ({searchTime.toFixed(3)} seconds)
            </div>
          )}
          
          {loading ? (
            <div style={styles.loadingContainer}>
              <div style={styles.loader}></div>
              <p>Searching...</p>
            </div>
          ) : error && results.length === 0 ? (
            <div style={{textAlign: 'center', padding: '20px', color: '#666'}}>
              {error}
            </div>
          ) : (
            <div>
              {results.map((result) => (
                <div key={result.id} style={styles.resultItem}>
                  <div 
                    onClick={() => handleViewDocument(result.id)}
                    style={{cursor: 'pointer'}}
                  >
                    <h3 style={{...styles.resultTitle, textDecoration: 'underline'}}>
                      {result.title || 'Untitled Document'}
                    </h3>
                    <div style={styles.resultMeta}>
                      Document #{result.id} | Score: {result.score.toFixed(4)}
                    </div>
                    <p style={styles.resultSnippet}>
                      {result.text ? result.text.substring(0, 150) + (result.text.length > 150 ? '...' : '') : 'No text available.'}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function DocumentPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const docId = window.location.pathname.split('/').pop() || '';
  
  const [document, setDocument] = useState<Document | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  
  const API_URL = 'http://127.0.0.1:8080';

  useEffect(() => {
    const stateDocument = location.state?.document;
    
    if (stateDocument) {
      setDocument(stateDocument);
      setLoading(false);
    } else {
      fetchDocument();
    }
  }, [docId, location.state]);
  
  const fetchDocument = async () => {
    try {
      setLoading(true);
      
      const response = await fetch(`${API_URL}/document/${docId}`, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      });
      
      if (!response.ok) {
        throw new Error(`API responded with status: ${response.status}`);
      }
      
      const data = await response.json();
      setDocument(data);
    } catch (error) {
      console.error('Error fetching document:', error);
      setError(`Failed to load document: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={styles.documentContainer}>
      {}
      <div style={styles.documentHeader}>
        <button 
          onClick={() => navigate(-1)}
          style={styles.backButton}
        >
          ←
        </button>
        <div>
            <Link to="/" style={{textDecoration: 'none', color: 'inherit'}}>
              <img 
                src={logoImage} 
                alt="Logo" 
                style={{ 
                  width: '150px', 
                  height: 'auto'
                }} 
              />
            </Link>
      </div>
      </div>

      {loading ? (
        <div style={styles.loadingContainer}>
          <div style={styles.loader}></div>
          <p>Loading document...</p>
        </div>
      ) : error ? (
        <div style={styles.errorContainer}>
          {error}
        </div>
      ) : document ? (
        <div style={styles.documentContent}>
          <h1 style={styles.documentTitle}>
            {document.title || 'Untitled Document'}
          </h1>
          
          <div style={styles.documentSection}>
            <h2 style={styles.documentSectionTitle}>Document Text</h2>
            <div style={styles.documentText}>
              {document.text || 'No text available.'}
            </div>
          </div>
          
          <div style={styles.documentFooter}>
            <span style={styles.documentId}>Document #{document.id}</span>
            <span style={styles.documentScore}>
              Score: {document.score.toFixed(4)}
            </span>
          </div>
        </div>
      ) : (
        <div style={{textAlign: 'center', color: '#666'}}>
          Document not found
        </div>
      )}
    </div>
  );
}

const styleElement = document.createElement('style');
styleElement.textContent = `
  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
`;
document.head.appendChild(styleElement);

export default function App() {
  return (
    <Router>
      <Routes>
        <Route path="/" element={<SearchPage />} />
        <Route path="/document/:id" element={<DocumentPage />} />
      </Routes>
    </Router>
  );
}