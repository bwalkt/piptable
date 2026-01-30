export interface GistFile {
  filename: string;
  content: string;
}

export interface CreateGistRequest {
  description: string;
  public: boolean;
  files: Record<string, { content: string }>;
}

export interface GistResponse {
  id: string;
  html_url: string;
  description: string;
  public: boolean;
  created_at: string;
  updated_at: string;
  files: Record<string, {
    filename: string;
    content?: string;
    raw_url: string;
    truncated?: boolean;
  }>;
}

// GitHub API base URL
const GITHUB_API_BASE = 'https://api.github.com';

// Create a new GitHub Gist
export async function createGist(
  files: GistFile[],
  description: string,
  isPublic: boolean = true,
  accessToken?: string
): Promise<GistResponse> {
  const filesObject: Record<string, { content: string }> = {};
  
  files.forEach(file => {
    filesObject[file.filename] = { content: file.content };
  });

  const payload: CreateGistRequest = {
    description,
    public: isPublic,
    files: filesObject,
  };

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'Accept': 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
  };

  // Add authorization header if token is provided
  if (accessToken) {
    headers['Authorization'] = `Bearer ${accessToken}`;
  }

  try {
    const response = await fetch(`${GITHUB_API_BASE}/gists`, {
      method: 'POST',
      headers,
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      
      if (response.status === 401) {
        throw new Error('Authentication failed. Please check your GitHub token.');
      } else if (response.status === 403) {
        throw new Error('Rate limit exceeded or insufficient permissions.');
      } else if (response.status === 422) {
        throw new Error('Invalid gist data. Please check your files and description.');
      } else {
        throw new Error(
          errorData.message || 
          `Failed to create gist: ${response.status} ${response.statusText}`
        );
      }
    }

    const gist = await response.json() as GistResponse;
    return gist;
  } catch (error) {
    if (error instanceof Error) {
      throw error;
    }
    throw new Error('Network error occurred while creating gist');
  }
}

// Fetch a GitHub Gist by ID
export async function fetchGist(gistId: string, accessToken?: string): Promise<GistResponse> {
  const headers: Record<string, string> = {
    'Accept': 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
  };

  if (accessToken) {
    headers['Authorization'] = `Bearer ${accessToken}`;
  }

  try {
    const response = await fetch(`${GITHUB_API_BASE}/gists/${gistId}`, {
      headers,
    });

    if (!response.ok) {
      if (response.status === 404) {
        throw new Error('Gist not found. Please check the ID and try again.');
      } else if (response.status === 403) {
        throw new Error('Access denied. The gist may be private or rate limited.');
      } else {
        throw new Error(`Failed to fetch gist: ${response.status} ${response.statusText}`);
      }
    }

    return await response.json() as GistResponse;
  } catch (error) {
    if (error instanceof Error) {
      throw error;
    }
    throw new Error('Network error occurred while fetching gist');
  }
}

// Extract gist ID from various GitHub URLs
export function extractGistId(url: string): string | null {
  try {
    // Handle various GitHub gist URL formats:
    // https://gist.github.com/username/abc123def456
    // https://gist.github.com/abc123def456
    // abc123def456
    
    const gistIdRegex = /(?:gist\.github\.com\/(?:[\w-]+\/)?)?([a-f0-9]{20,32})/i;
    const match = url.match(gistIdRegex);
    
    return match ? match[1] : null;
  } catch {
    return null;
  }
}

// Create a gist from playground code
export async function createPlaygroundGist(
  code: string,
  title?: string,
  description?: string,
  isPublic: boolean = true,
  accessToken?: string
): Promise<GistResponse> {
  const filename = title ? `${title.replace(/[^a-zA-Z0-9-_]/g, '_')}.pip` : 'playground.pip';
  const gistDescription = description || `PipTable Playground - ${title || 'Untitled Script'}`;
  
  return createGist([
    {
      filename,
      content: code,
    },
    {
      filename: 'README.md',
      content: createGistReadme(title, description, code),
    }
  ], gistDescription, isPublic, accessToken);
}

// Generate README content for the gist
function createGistReadme(title?: string, description?: string, code?: string): string {
  const lines = [
    '# PipTable Playground Script',
    '',
    title ? `## ${title}` : '## Untitled Script',
    '',
  ];

  if (description) {
    lines.push(description, '');
  }

  lines.push(
    '## About PipTable',
    '',
    'This script is written in [PipTable](https://piptable.com) - a domain-specific language for data manipulation and analysis.',
    '',
    '### Usage',
    '',
    '1. Copy the `.pip` file content',
    '2. Open the [PipTable Playground](https://piptable.com/playground)',
    '3. Paste and run the code',
    '',
    '### Features Used',
    ''
  );

  if (code) {
    // Analyze code for common features
    const features = [];
    if (code.includes('import')) features.push('- Data import');
    if (code.includes('export')) features.push('- Data export');
    if (code.includes('query(')) features.push('- SQL queries');
    if (code.includes('join')) features.push('- Data joins');
    if (code.includes('for ') || code.includes('while ')) features.push('- Control flow');
    if (code.includes('function ')) features.push('- Custom functions');
    
    if (features.length > 0) {
      lines.push(...features, '');
    }
  }

  lines.push(
    '---',
    '',
    '*Created with [PipTable Playground](https://piptable.com/playground)*'
  );

  return lines.join('\n');
}

// Load playground code from a gist
export async function loadPlaygroundGist(
  gistUrl: string,
  accessToken?: string
): Promise<{ code: string; title: string; description: string }> {
  const gistId = extractGistId(gistUrl);
  
  if (!gistId) {
    throw new Error('Invalid GitHub Gist URL. Please provide a valid gist URL or ID.');
  }

  const gist = await fetchGist(gistId, accessToken);
  
  // Look for .pip file first, then any text file
  let codeFile = Object.values(gist.files).find(file => 
    file.filename.endsWith('.pip') || 
    file.filename.endsWith('.txt') ||
    file.filename.endsWith('.piptable')
  );

  // If no specific file found, use the first file
  if (!codeFile) {
    codeFile = Object.values(gist.files)[0];
  }

  if (!codeFile) {
    throw new Error('No code files found in the gist.');
  }

  // Handle truncated or missing content by fetching the raw content
  let content = codeFile.content ?? '';
  if (codeFile.truncated || !content) {
    if (!codeFile.raw_url) {
      throw new Error('Gist file content is truncated and no raw URL is available.');
    }
    try {
      const headers: Record<string, string> = {};
      if (accessToken) {
        headers['Authorization'] = `Bearer ${accessToken}`;
      }
      const response = await fetch(codeFile.raw_url, { headers });
      if (!response.ok) {
        throw new Error(`Failed to fetch raw content: ${response.status}`);
      }
      content = await response.text();
    } catch (error) {
      throw new Error(`Failed to load complete file content: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  return {
    code: content,
    title: gist.description || 'Imported from Gist',
    description: `Imported from GitHub Gist: ${gist.html_url}`,
  };
}

// Validate GitHub access token
export async function validateGitHubToken(token: string): Promise<boolean> {
  try {
    const response = await fetch(`${GITHUB_API_BASE}/user`, {
      headers: {
        'Authorization': `Bearer ${token}`,
        'Accept': 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
      },
    });

    return response.ok;
  } catch {
    return false;
  }
}

// Get rate limit information
export async function getRateLimit(accessToken?: string): Promise<{
  remaining: number;
  limit: number;
  reset: Date;
}> {
  const headers: Record<string, string> = {
    'Accept': 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
  };

  if (accessToken) {
    headers['Authorization'] = `Bearer ${accessToken}`;
  }

  try {
    const response = await fetch(`${GITHUB_API_BASE}/rate_limit`, { headers });
    
    if (!response.ok) {
      throw new Error('Failed to get rate limit information');
    }

    const data = await response.json();
    const core = data.rate;

    return {
      remaining: core.remaining,
      limit: core.limit,
      reset: new Date(core.reset * 1000),
    };
  } catch (error) {
    // Return conservative values if API call fails - assume limited quota
    // Note: remaining: 60 indicates unknown state, not rate-limited
    return {
      remaining: 60, // Conservative assumption when status unknown
      limit: 60,
      reset: new Date(),
    };
  }
}
