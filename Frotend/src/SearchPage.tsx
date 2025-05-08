import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import logoImage from '../logo.jpg';
import styles from '../styles/App.css';

interface Document {
  id: string;
  title: string;
  text: string;
  authors: string[];
  score: number;
}

interface Stats {
  document_count: number;
  vocabulary_size: number;
}

export default function SearchPage() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<Document[]>([]);
  const [loading, setLoading] = useState(false);
  const [apiStatus, setApiStatus] = useState('checking');
  const [stats, setStats] = useState<Stats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [resultCount, setResultCount] = useState(10);
  const [searchTime, setSearchTime] = useState<number | null>(null);
  const navigate = useNavigate();

  const API_URL = 'http://127.0.0.1:8080';

  useEffect(() => {
    checkApiConnection();
  }, []);

  const checkApiConnection = async () => {
    try {
      const response = await fetch(`${API_URL}/stats`);
      if (response.ok) {
        const statsData = await response.json();
        setStats(statsData);
        setApiStatus('connected');
      } else {
        throw new Error('API error');
      }
    } catch {
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
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query, limit: resultCount }),
      });

      if (!response.ok) throw new Error(`API responded with status: ${response.status}`);

      const data = await response.json();
      const endTime = performance.now();
      setSearchTime((endTime - startTime) / 1000);
      setResults(data);
      if (data.length === 0) setError(`No results found for "${query}"`);
    } catch (err) {
      setError(`Search failed: ${(err as Error).message}`);
    } finally {
      setLoading(false);
    }
  };

  const handleViewDocument = (docId: string) => {
    navigate(`/document/${docId}`, { state: { document: results.find(doc => doc.id === docId) } });
  };

  return (
    <div className={styles.container}>
      <div className={styles.centerContainer}>
        <div className={styles.logoContainer}>
          <img src={logoImage} alt="Logo" className={styles.logoImage} />
        </div>

        <div className={styles.searchContainer}>
          {apiStatus === 'error' && (
            <div className={styles.errorContainer}>
              <div>⚠️ API Connection Failed</div>
              <p>{error}</p>
              <button onClick={checkApiConnection}>Retry Connection</button>
            </div>
          )}

          <div className={styles.searchForm}>
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search documents..."
              className={styles.searchInput}
              onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
              disabled={apiStatus !== 'connected'}
            />
            <button
              onClick={handleSearch}
              disabled={loading || apiStatus !== 'connected'}
              className={`${styles.searchButton} ${apiStatus !== 'connected' ? styles.disabledButton : ''}`}
            >
              {loading ? 'Searching...' : 'Search'}
            </button>
          </div>

          <div className={styles.searchStats}>
            {stats && <span>Indexed: {stats.document_count} | Vocab: {stats.vocabulary_size}</span>}
            <div>
              <label htmlFor="resultCount">Results:</label>
              <select
                id="resultCount"
                value={resultCount}
                onChange={(e) => setResultCount(Number(e.target.value))}
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

      {(loading || results.length > 0 || error) && (
        <div className={styles.resultsContainer}>
          {searchTime !== null && results.length > 0 && (
            <div className={styles.searchTime}>
              About {results.length} results ({searchTime.toFixed(3)} sec)
            </div>
          )}
          {loading ? (
            <div className={styles.loadingContainer}>
              <div className={styles.loader}></div>
              <p>Searching...</p>
            </div>
          ) : error ? (
            <div className={styles.errorMessage}>{error}</div>
          ) : (
            results.map((result) => (
              <div key={result.id} className={styles.resultItem} onClick={() => handleViewDocument(result.id)}>
                <h3 className={styles.resultTitle}>{result.title || 'Untitled'}</h3>
                <div className={styles.resultMeta}>Document #{result.id} | Score: {result.score.toFixed(4)}</div>
                <div className={styles.resultAuthor}>{result.authors?.join(', ') || 'Unknown Author'}</div>
                <p className={styles.resultSnippet}>
                  {result.text?.substring(0, 150) + (result.text.length > 150 ? '...' : '')}
                </p>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
