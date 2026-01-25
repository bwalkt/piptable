# PipTable Playground

Interactive playground for experimenting with PipTable DSL code.

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Features

- **CodeMirror 6 Editor**: Modern code editor with syntax highlighting
- **Example Gallery**: Pre-built examples to learn from
- **Live Execution**: Run PipTable code in the browser (WASM - coming soon)
- **Responsive Design**: Works on desktop and mobile devices
- **Dark/Light Themes**: Toggle between color schemes

## Structure

```
playground/
├── index.html           # Main HTML file
├── src/
│   └── main.ts         # Application entry point
├── examples/           # Example code snippets
├── dist/              # Build output
└── vite.config.ts     # Build configuration
```

## Integration with mdBook

The playground is linked from the main documentation. After building both mdBook and the playground:

```bash
# Build documentation
cd docs
mdbook build

# Build playground
cd playground
npm run build

# Files are served from docs/book/
```

## Planned Features

- [ ] PipTable syntax highlighting (Lezer grammar)
- [ ] WASM execution engine
- [ ] Autocomplete support
- [ ] Error diagnostics
- [ ] Share/save functionality
- [ ] More examples

## License

Same as PipTable project.