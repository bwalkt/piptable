# Planned: Parameter Performance Considerations

Note: This document outlines planned analysis and optimizations. It is not current behavior.

This document analyzes performance aspects of parameter passing in PipTable DSL and suggests optimizations.

## Current Implementation Analysis

### Parameter Binding Performance

The current parameter implementation creates new variable bindings for each function call:

```rust
// Current approach (simplified)
pub struct Param {
    pub name: String,
    pub mode: ParamMode,
}
```

**Performance Characteristics:**
- ✅ Simple and correct implementation
- ⚠️ String allocation for each parameter name
- ⚠️ Linear search through parameter list during binding
- ⚠️ No caching of parameter metadata

### ByRef Reference Tracking

ByRef parameters require maintaining references to original variables:

**Current Overhead:**
- Reference validation at call time
- Additional memory for reference tracking
- Potential for reference chains in nested calls

## Performance Optimization Opportunities

### 1. Parameter Metadata Caching

**Current Issue:** Parameter information is parsed and validated on every function call.

**Proposed Solution:** Cache compiled parameter metadata in function definitions.

```rust
// Enhanced parameter representation
#[derive(Clone)]
pub struct CompiledParam {
    pub name_id: ParameterNameId,  // Interned string ID
    pub mode: ParamMode,
    pub position: u8,              // Parameter position for fast lookup
}

pub struct FunctionMetadata {
    pub params: Vec<CompiledParam>,
    pub param_count: u8,
    pub has_byref_params: bool,    // Fast check for reference handling
}
```

**Benefits:**
- Faster parameter binding (O(1) vs O(n) lookup)
- Reduced memory allocations
- Pre-computed metadata for validation

### 2. String Interning for Parameter Names

**Current Issue:** Parameter names are stored as `String`, causing allocations.

**Proposed Solution:** Use string interning for parameter names.

```rust
use string_interner::{StringInterner, DefaultSymbol};

pub type ParameterNameId = DefaultSymbol;

// Global parameter name interner
static PARAM_NAMES: Lazy<Mutex<StringInterner>> = Lazy::new(|| {
    Mutex::new(StringInterner::new())
});

impl Param {
    pub fn new(name: &str, mode: ParamMode) -> Self {
        let name_id = PARAM_NAMES.lock().unwrap().get_or_intern(name);
        Self { name_id, mode }
    }
}
```

**Benefits:**
- Reduced memory usage (especially for common parameter names like `x`, `y`, `data`)
- Faster parameter name comparisons (integer comparison vs string comparison)
- Better cache locality

### 3. Optimized ByRef Validation

**Current Issue:** ByRef validation may involve expensive lvalue analysis.

**Proposed Solution:** Pre-categorize expressions for faster validation.

```rust
#[derive(Clone, Copy)]
pub enum ExpressionKind {
    Variable,          // Simple variable reference
    ArrayElement,      // Array indexing operation
    ObjectField,       // Field access operation  
    Expression,        // Complex expression (invalid for ByRef)
}

impl Expr {
    pub fn kind(&self) -> ExpressionKind {
        match self {
            Expr::Variable(_) => ExpressionKind::Variable,
            Expr::ArrayIndex { .. } => ExpressionKind::ArrayElement,
            Expr::FieldAccess { .. } => ExpressionKind::ObjectField,
            _ => ExpressionKind::Expression,
        }
    }
    
    pub fn is_valid_byref_target(&self) -> bool {
        !matches!(self.kind(), ExpressionKind::Expression)
    }
}
```

**Benefits:**
- O(1) ByRef validation instead of recursive AST traversal
- Clear categorization of expression types
- Potential for compile-time validation in future

## Parameter Array Performance (Future Enhancement)

### Current Challenge

When parameter arrays are implemented, they may face performance issues with large argument lists:

```vb
' Potential performance concern with many arguments
function sum(ParamArray values)
    dim total = 0
    for each val in values
        total = total + val
    next
    return total
end function

' Call with many arguments
dim result = sum(1, 2, 3, ..., 10000)  ' 10k arguments
```

### Proposed Optimizations

#### 1. Chunked Parameter Processing

```rust
pub struct ParamArray {
    chunks: Vec<Vec<Value>>,     // Process arguments in chunks
    chunk_size: usize,           // Configurable chunk size
}

impl ParamArray {
    pub fn new(args: Vec<Value>) -> Self {
        const DEFAULT_CHUNK_SIZE: usize = 1024;
        let chunks = args.chunks(DEFAULT_CHUNK_SIZE)
                        .map(|chunk| chunk.to_vec())
                        .collect();
        Self { chunks, chunk_size: DEFAULT_CHUNK_SIZE }
    }
    
    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        self.chunks.iter().flat_map(|chunk| chunk.iter())
    }
}
```

#### 2. Lazy Parameter Array Evaluation

```rust
pub enum ParamArray {
    Eager(Vec<Value>),                    // Small arrays
    Lazy(Box<dyn Iterator<Item = Value>>), // Large arrays with lazy evaluation
}

impl ParamArray {
    pub fn new(args: Vec<Value>) -> Self {
        if args.len() < 100 {
            Self::Eager(args)
        } else {
            Self::Lazy(Box::new(args.into_iter()))
        }
    }
}
```

#### 3. Memory-Mapped Large Parameter Arrays

For extremely large parameter lists, consider memory-mapping:

```rust
use memmap2::Mmap;

pub struct LargeParamArray {
    mmap: Mmap,                    // Memory-mapped parameter data
    value_offsets: Vec<usize>,     // Offsets to individual values
}
```

## Benchmarking Framework

### Proposed Performance Tests

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[bench]
    fn bench_parameter_binding_small() {
        // Test parameter binding with 1-5 parameters
        let params = create_test_params(3);
        let args = create_test_args(3);
        
        let start = Instant::now();
        for _ in 0..10000 {
            bind_parameters(&params, &args);
        }
        let duration = start.elapsed();
        
        assert!(duration.as_millis() < 100, "Parameter binding too slow");
    }

    #[bench] 
    fn bench_parameter_binding_large() {
        // Test parameter binding with many parameters
        let params = create_test_params(20);
        let args = create_test_args(20);
        
        let start = Instant::now();
        for _ in 0..1000 {
            bind_parameters(&params, &args);
        }
        let duration = start.elapsed();
        
        assert!(duration.as_millis() < 500, "Large parameter binding too slow");
    }

    #[bench]
    fn bench_byref_validation() {
        let expressions = create_byref_test_expressions();
        
        let start = Instant::now();
        for expr in &expressions {
            let _ = expr.is_valid_byref_target();
        }
        let duration = start.elapsed();
        
        assert!(duration.as_nanos() < 1000, "ByRef validation too slow");
    }

    #[bench]
    fn bench_param_array_iteration() {
        let large_array = create_param_array(10000);
        
        let start = Instant::now();
        let mut sum = 0;
        for value in large_array.iter() {
            sum += value.as_int().unwrap_or(0);
        }
        let duration = start.elapsed();
        
        assert!(duration.as_millis() < 50, "ParamArray iteration too slow");
    }
}
```

### Performance Metrics to Track

1. **Parameter Binding Time**
   - Time to bind arguments to parameters
   - Scale with parameter count

2. **ByRef Validation Time**  
   - Time to validate ByRef arguments
   - Should be O(1) for simple cases

3. **Memory Usage**
   - Memory overhead per parameter
   - Memory growth with parameter count

4. **Function Call Overhead**
   - Total overhead of parameter processing
   - Comparison with/without optimizations

## Implementation Priority

### Phase 1: Basic Optimizations (Low Hanging Fruit)
- [ ] String interning for parameter names
- [ ] Pre-computed parameter metadata caching
- [ ] Fast ByRef validation with expression categorization

### Phase 2: Advanced Optimizations
- [ ] Chunked parameter processing
- [ ] Lazy parameter array evaluation
- [ ] Comprehensive benchmarking suite

### Phase 3: Specialized Optimizations  
- [ ] Memory-mapped large parameter arrays
- [ ] SIMD-optimized parameter processing for numeric arrays
- [ ] Custom allocators for parameter-heavy workloads

## Monitoring and Profiling

### Recommended Profiling Tools

1. **Criterion.rs** - For micro-benchmarks of parameter operations
2. **pprof** - For CPU profiling of parameter-heavy code
3. **Valgrind/Heaptrack** - For memory usage analysis
4. **perf** - For low-level performance analysis on Linux

### Key Performance Indicators

- Function call overhead: < 1μs for simple functions
- Parameter binding: < 100ns per parameter
- ByRef validation: < 10ns per validation
- Memory usage: < 50 bytes overhead per function call

This performance analysis provides a roadmap for optimizing parameter handling while maintaining the correctness and simplicity of the current implementation.
