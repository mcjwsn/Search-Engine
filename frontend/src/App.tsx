import { useState, useEffect } from 'react';

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

export default function App() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<Document[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [selectedDocument, setSelectedDocument] = useState<Document | null>(null);
  const [resultCount, setResultCount] = useState<number>(10);
  const [apiStatus, setApiStatus] = useState<string>('checking');
  const [stats, setStats] = useState<Stats | null>(null);
  const [error, setError] = useState<string | null>(null);

  const API_URL = 'http://127.0.0.1:8080';

  // Check API connection on component mount
  useEffect(() => {
    checkApiConnection();
  }, []);

  const checkApiConnection = async () => {
    try {
      setApiStatus('checking');
      
      // Try to get stats to check if API is running
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

  // Handle search submission
  const handleSearch = async () => {
    if (!query.trim()) return;
    
    setLoading(true);
    setResults([]);
    setSelectedDocument(null);
    setError(null);
    
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
      setResults(data);
      
      if (data.length === 0) {
        setError(`No results found for "${query}"`);
      }
    } catch (error) {
      console.error('Error searching:', error);
      setError(`Search failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="max-w-6xl mx-auto p-6 font-sans">
      <h1 className="text-3xl font-bold text-center mb-6">CISI Document Search</h1>
      
      {/* API Status */}
      <div className={`mb-4 p-3 rounded-md text-sm ${
        apiStatus === 'connected' ? 'bg-green-100 text-green-800' : 
        apiStatus === 'error' ? 'bg-red-100 text-red-800' : 
        'bg-yellow-100 text-yellow-800'
      }`}>
        <div className="flex items-center gap-2">
          <div className={`w-3 h-3 rounded-full ${
            apiStatus === 'connected' ? 'bg-green-500' : 
            apiStatus === 'error' ? 'bg-red-500' : 
            'bg-yellow-500'
          }`}></div>
          <span className="font-medium">
            {apiStatus === 'connected' ? 'API Connected' : 
             apiStatus === 'error' ? 'API Connection Failed' : 
             'Checking API Connection...'}
          </span>
        </div>
        
        {stats && apiStatus === 'connected' && (
          <div className="mt-2 text-sm">
            <span className="mr-4">Documents: {stats.document_count}</span>
            <span>Vocabulary terms: {stats.vocabulary_size}</span>
          </div>
        )}
        
        {apiStatus === 'error' && (
          <div className="mt-2">
            <p>{error}</p>
            <button 
              onClick={checkApiConnection}
              className="mt-2 px-3 py-1 bg-white text-red-700 rounded-md border border-red-300 text-sm hover:bg-red-50"
            >
              Retry Connection
            </button>
          </div>
        )}
      </div>
      
      {/* Search Box */}
      <div className="bg-gray-50 p-4 rounded-lg border border-gray-200 mb-6">
        <div className="flex gap-2 mb-3">
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Enter your search query..."
            className="flex-1 p-2 border border-gray-300 rounded-md"
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
            className={`px-4 py-2 rounded-md text-white font-medium ${
              apiStatus !== 'connected' ? 'bg-gray-400 cursor-not-allowed' : 'bg-blue-600 hover:bg-blue-700'
            }`}
          >
            {loading ? 'Searching...' : 'Search'}
          </button>
        </div>
        
        <div className="flex items-center">
          <label htmlFor="resultCount" className="mr-2 text-sm">Results to show:</label>
          <select
            id="resultCount"
            value={resultCount}
            onChange={(e) => setResultCount(Number(e.target.value))}
            className="border border-gray-300 rounded-md p-1 text-sm"
            disabled={apiStatus !== 'connected'}
          >
            <option value={5}>5</option>
            <option value={10}>10</option>
            <option value={25}>25</option>
            <option value={50}>50</option>
          </select>
        </div>
      </div>

      {/* Results and Document View */}
      <div className="grid md:grid-cols-2 gap-6">
        {/* Results List */}
        <div>
          <h2 className="text-xl font-semibold mb-3">
            Search Results {results.length > 0 && `(${results.length})`}
          </h2>
          
          {loading ? (
            <div className="text-center p-8 border rounded-md bg-white">
              <div className="inline-block animate-spin rounded-full h-8 w-8 border-4 border-gray-200 border-t-blue-600"></div>
              <p className="mt-2 text-gray-600">Searching...</p>
            </div>
          ) : error && results.length === 0 ? (
            <div className="text-center p-6 border rounded-md bg-white text-gray-600">
              {error}
            </div>
          ) : results.length > 0 ? (
            <div className="space-y-3 max-h-[calc(100vh-300px)] overflow-y-auto pr-2">
              {results.map((result) => (
                <div 
                  key={result.id}
                  onClick={() => setSelectedDocument(result)}
                  className={`p-3 border rounded-md cursor-pointer bg-white hover:bg-blue-50 transition-colors ${
                    selectedDocument?.id === result.id ? 'border-l-4 border-l-blue-600' : ''
                  }`}
                >
                  <h3 className="font-medium mb-1">{result.title || 'Untitled Document'}</h3>
                  <div className="text-sm text-gray-600">
                    Authors: {result.authors && result.authors.length > 0 ? result.authors.join(', ') : 'Unknown'}
                  </div>
                  <div className="flex justify-between mt-2 text-sm">
                    <span>Doc #{result.id}</span>
                    <span className="bg-blue-100 text-blue-800 px-2 py-1 rounded-full text-xs">
                      Score: {result.score.toFixed(4)}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          ) : null}
        </div>

        {/* Document Viewer */}
        <div>
          <h2 className="text-xl font-semibold mb-3">Document Details</h2>
          
          {selectedDocument ? (
            <div className="border rounded-md p-4 bg-white">
              <h3 className="text-lg font-medium pb-2 border-b">
                {selectedDocument.title || 'Untitled Document'}
              </h3>
              
              <div className="my-3">
                <h4 className="font-medium mb-1">Authors</h4>
                {selectedDocument.authors && selectedDocument.authors.length > 0 ? (
                  <ul className="list-disc pl-5">
                    {selectedDocument.authors.map((author, index) => (
                      <li key={index}>{author}</li>
                    ))}
                  </ul>
                ) : (
                  <p className="text-gray-600">Unknown Author</p>
                )}
              </div>
              
              <div className="my-3">
                <h4 className="font-medium mb-1">Text</h4>
                <p className="whitespace-pre-line text-gray-800 max-h-64 overflow-y-auto border p-2 rounded-md bg-gray-50">
                  {selectedDocument.text || 'No text available.'}
                </p>
              </div>
              
              <div className="bg-gray-100 p-3 mt-3 rounded-md flex justify-between items-center">
                <span className="text-sm text-gray-600">Document #{selectedDocument.id}</span>
                <span className="bg-blue-100 text-blue-800 px-2 py-1 rounded-full text-xs">
                  Score: {selectedDocument.score.toFixed(4)}
                </span>
              </div>
            </div>
          ) : (
            <div className="border rounded-md p-8 bg-white text-center text-gray-600">
              Select a document from the results to view details
            </div>
          )}
        </div>
      </div>
    </div>
  );
}