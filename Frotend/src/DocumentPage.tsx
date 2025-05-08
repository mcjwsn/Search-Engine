import { useEffect, useState } from 'react';
import { useNavigate, useLocation, Link } from 'react-router-dom';
import '../styles/App.css';
import logoImage from '../logo.jpg';

interface Document {
  id: string;
  title: string;
  text: string;
  authors: string[];
  score: number;
}

export default function DocumentPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const docId = window.location.pathname.split('/').pop() || '';
  const [document, setDocument] = useState<Document | null>(null);
  const [loading, setLoading] = useState(true);
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
      const response = await fetch(`${API_URL}/document/${docId}`);
      if (!response.ok) throw new Error(`API error ${response.status}`);
      const data = await response.json();
      setDocument(data);
    } catch (err) {
      setError('Failed to load document');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="document-container">
      <div className="document-header">
        <button className="back-button" onClick={() => navigate(-1)}>←</button>
        <Link to="/">
          <img src={logoImage} alt="Logo" className="logo-small" />
        </Link>
      </div>

      {loading ? (
        <div className="loading-container">
          <div className="loader" />
          <p>Loading document...</p>
        </div>
      ) : error ? (
        <div className="error-container">{error}</div>
      ) : document ? (
        <div className="document-content">
          <h1 className="document-title">{document.title || 'Untitled Document'}</h1>

          <div className="document-section">
            <h2 className="document-section-title">Authors</h2>
            {document.authors?.length ? (
              <ul>
                {document.authors.map((author, idx) => <li key={idx}>{author}</li>)}
              </ul>
            ) : (
              <p className="text-muted">Unknown Author</p>
            )}
          </div>

          <div className="document-section">
            <h2 className="document-section-title">Document Text</h2>
            <div className="document-text">{document.text || 'No text available.'}</div>
          </div>

          <div className="document-footer">
            <span className="document-id">Document #{document.id}</span>
            <span className="document-score">Score: {document.score.toFixed(4)}</span>
          </div>
        </div>
      ) : (
        <p>Document not found.</p>
      )}
    </div>
  );
}
