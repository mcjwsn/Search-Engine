import { useState, useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Link, useNavigate, useLocation } from 'react-router-dom';
import logoImage from './logo.jpg';
import './App.css';


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
    <div className="container">
      {}
      <div className="centerContainer">
        {}
        <div className="logo-container">
          <img 
            src={logoImage} 
            alt="Logo" 
            className="logoImage"
          />
        </div>

        {}
        <div className="searchContainer">
          {}
          {apiStatus === 'error' && (
            <div className="errorContainer">
              <div>
                <span>⚠️ API Connection Failed</span>
              </div>
              <div>
                <p>{error}</p>
                <button 
                  onClick={checkApiConnection}
                  className="retryButton"
                >
                  Retry Connection
                </button>
              </div>
            </div>
          )}

          <div className="searchForm">
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search documents..."
              className="searchInput"
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
              className={`searchButton ${apiStatus !== 'connected' ? 'disabledButton' : ''}`}
            >
              {loading ? 'Searching...' : 'Search'}
            </button>
          </div>
          
          <div className="searchStats">
            <div>
              {stats && apiStatus === 'connected' && (
                <span>Indexed documents: {stats.document_count} | Vocabulary: {stats.vocabulary_size}</span>
              )}
            </div>
            <div>
              <label htmlFor="resultCount" className="resultCountLabel">Results:</label>
              <select
                id="resultCount"
                value={resultCount}
                onChange={(e) => setResultCount(Number(e.target.value))}
                className="resultCountSelect"
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
        <div className="resultsContainer">
          {}
          {searchTime !== null && results.length > 0 && (
            <div className="searchTime">
              About {results.length} results ({searchTime.toFixed(3)} seconds)
            </div>
          )}
          
          {loading ? (
            <div className="loadingContainer">
              <div className="loader"></div>
              <p>Searching...</p>
            </div>
          ) : error && results.length === 0 ? (
            <div className="errorMessage">
              {error}
            </div>
          ) : (
            <div>
              {results.map((result) => (
                <div key={result.id} className="resultItem">
                  <div 
                    onClick={() => handleViewDocument(result.id)}
                    className="resultClickable"
                  >
                    <h3 className="resultTitle">
                      {result.title || 'Untitled Document'}
                    </h3>
                    <div className="resultMeta">
                      Document #{result.id} | Score: {result.score.toFixed(4)}
                    </div>
                    <p className="resultSnippet">
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
    <div className="documentContainer">
      {}
      <div className="documentHeader">
        <button 
          onClick={() => navigate(-1)}
          className="backButton"
        >
          ←
        </button>
        <div>
            <Link to="/" className="logoLink">
              <img 
                src={logoImage} 
                alt="Logo" 
                className="documentLogoImage"
              />
            </Link>
      </div>
      </div>

      {loading ? (
        <div className="loadingContainer">
          <div className="loader"></div>
          <p>Loading document...</p>
        </div>
      ) : error ? (
        <div className="errorContainer">
          {error}
        </div>
      ) : document ? (
        <div className="documentContent">
          <h1 className="documentTitle">
            {document.title || 'Untitled Document'}
          </h1>
          
          <div className="documentSection">
            <h2 className="documentSectionTitle">Document Text</h2>
            <div className="documentText">
              {document.text || 'No text available.'}
            </div>
          </div>
          
          <div className="documentFooter">
            <span className="documentId">Document #{document.id}</span>
            <span className="documentScore">
              Score: {document.score.toFixed(4)}
            </span>
          </div>
        </div>
      ) : (
        <div className="notFoundMessage">
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