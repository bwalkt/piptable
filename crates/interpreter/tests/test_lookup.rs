//! Tests for lookup functions (VLOOKUP, HLOOKUP, INDEX, MATCH, XLOOKUP)

use piptable_interpreter::Interpreter;

#[tokio::test]
async fn test_vlookup_exact_match() {
    let mut interpreter = Interpreter::new();
    
    // Create a test table with products and prices
    let script = r#"
        # Create a product table
        products = [
            ["Apple", 1.50, 100],
            ["Banana", 0.75, 200],
            ["Cherry", 2.00, 150],
            ["Date", 3.50, 50]
        ]
        
        # Test exact match
        price = vlookup("Banana", products, 2, false)
        quantity = vlookup("Cherry", products, 3, false)
        
        # Test not found
        not_found = vlookup("Grape", products, 2, false)
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let price = interpreter.get_value("price").await.unwrap();
    assert_eq!(price.to_string(), "0.75");
    
    let quantity = interpreter.get_value("quantity").await.unwrap();
    assert_eq!(quantity.to_string(), "150");
    
    let not_found = interpreter.get_value("not_found").await.unwrap();
    assert_eq!(not_found.to_string(), "#N/A");
}

#[tokio::test]
async fn test_hlookup_exact_match() {
    let mut interpreter = Interpreter::new();
    
    // Create a test table with quarterly data
    let script = r#"
        # Create quarterly sales table
        quarterly = [
            ["Product", "Q1", "Q2", "Q3", "Q4"],
            ["Sales", 100, 150, 120, 180],
            ["Costs", 80, 100, 90, 120]
        ]
        
        # Test exact match
        q2_sales = hlookup("Q2", quarterly, 2, false)
        q4_costs = hlookup("Q4", quarterly, 3, false)
        
        # Test not found
        not_found = hlookup("Q5", quarterly, 2, false)
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let q2_sales = interpreter.get_value("q2_sales").await.unwrap();
    assert_eq!(q2_sales.to_string(), "150");
    
    let q4_costs = interpreter.get_value("q4_costs").await.unwrap();
    assert_eq!(q4_costs.to_string(), "120");
    
    let not_found = interpreter.get_value("not_found").await.unwrap();
    assert_eq!(not_found.to_string(), "#N/A");
}

#[tokio::test]
async fn test_index_function() {
    let mut interpreter = Interpreter::new();
    
    let script = r#"
        # Create a 2D array
        data = [
            [10, 20, 30],
            [40, 50, 60],
            [70, 80, 90]
        ]
        
        # Test 2D indexing
        val1 = index(data, 2, 3)  # Row 2, Column 3 = 60
        val2 = index(data, 3, 1)  # Row 3, Column 1 = 70
        
        # Test 1D indexing (return entire row)
        row2 = index(data, 2)  # Should return [40, 50, 60]
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let val1 = interpreter.get_value("val1").await.unwrap();
    assert_eq!(val1.to_string(), "60");
    
    let val2 = interpreter.get_value("val2").await.unwrap();
    assert_eq!(val2.to_string(), "70");
    
    let row2 = interpreter.get_value("row2").await.unwrap();
    assert!(row2.to_string().contains("40"));
    assert!(row2.to_string().contains("50"));
    assert!(row2.to_string().contains("60"));
}

#[tokio::test]
async fn test_match_exact() {
    let mut interpreter = Interpreter::new();
    
    let script = r#"
        # Create arrays for testing
        fruits = ["Apple", "Banana", "Cherry", "Date"]
        numbers = [10, 20, 30, 40, 50]
        
        # Test exact match (match_type = 0)
        pos1 = match("Banana", fruits, 0)  # Position 2
        pos2 = match("Date", fruits, 0)    # Position 4
        pos3 = match(30, numbers, 0)       # Position 3
        
        # Test not found
        not_found = match("Grape", fruits, 0)
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let pos1 = interpreter.get_value("pos1").await.unwrap();
    assert_eq!(pos1.to_string(), "2");
    
    let pos2 = interpreter.get_value("pos2").await.unwrap();
    assert_eq!(pos2.to_string(), "4");
    
    let pos3 = interpreter.get_value("pos3").await.unwrap();
    assert_eq!(pos3.to_string(), "3");
    
    let not_found = interpreter.get_value("not_found").await.unwrap();
    assert_eq!(not_found.to_string(), "#N/A");
}

#[tokio::test]
async fn test_match_less_than_or_equal() {
    let mut interpreter = Interpreter::new();
    
    let script = r#"
        # Create sorted array for testing
        sorted_nums = [10, 20, 30, 40, 50]
        
        # Test less than or equal (match_type = 1)
        pos1 = match(25, sorted_nums, 1)  # Should return position 2 (20)
        pos2 = match(30, sorted_nums, 1)  # Should return position 3 (30)
        pos3 = match(55, sorted_nums, 1)  # Should return position 5 (50)
        pos4 = match(5, sorted_nums, 1)   # Should return #N/A
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let pos1 = interpreter.get_value("pos1").await.unwrap();
    assert_eq!(pos1.to_string(), "2");
    
    let pos2 = interpreter.get_value("pos2").await.unwrap();
    assert_eq!(pos2.to_string(), "3");
    
    let pos3 = interpreter.get_value("pos3").await.unwrap();
    assert_eq!(pos3.to_string(), "5");
    
    let pos4 = interpreter.get_value("pos4").await.unwrap();
    assert_eq!(pos4.to_string(), "#N/A");
}

#[tokio::test]
async fn test_xlookup_basic() {
    let mut interpreter = Interpreter::new();
    
    let script = r#"
        # Create lookup and return arrays
        product_names = ["Apple", "Banana", "Cherry", "Date"]
        product_prices = [1.50, 0.75, 2.00, 3.50]
        
        # Test basic XLOOKUP
        price1 = xlookup("Banana", product_names, product_prices)
        price2 = xlookup("Date", product_names, product_prices)
        
        # Test with if_not_found parameter
        price3 = xlookup("Grape", product_names, product_prices, "Not Found")
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let price1 = interpreter.get_value("price1").await.unwrap();
    assert_eq!(price1.to_string(), "0.75");
    
    let price2 = interpreter.get_value("price2").await.unwrap();
    assert_eq!(price2.to_string(), "3.5");
    
    let price3 = interpreter.get_value("price3").await.unwrap();
    assert_eq!(price3.to_string(), "Not Found");
}

#[tokio::test]
async fn test_vlookup_with_headers() {
    let mut interpreter = Interpreter::new();
    
    let script = r#"
        # Import data with headers
        import "test_data.csv" into sales_data
        
        # Assume sales_data has columns: Product, Price, Quantity
        # Use VLOOKUP to find prices
        apple_price = vlookup("Apple", sales_data, 2, false)
    "#;
    
    // Create test CSV file
    std::fs::write(
        "test_data.csv",
        "Product,Price,Quantity\nApple,1.50,100\nBanana,0.75,200\nCherry,2.00,150"
    ).unwrap();
    
    interpreter.execute(script).await.unwrap();
    
    let apple_price = interpreter.get_value("apple_price").await.unwrap();
    assert_eq!(apple_price.to_string(), "1.5");
    
    // Clean up test file
    std::fs::remove_file("test_data.csv").ok();
}

#[tokio::test]
async fn test_index_match_combination() {
    let mut interpreter = Interpreter::new();
    
    let script = r#"
        # Create a product table
        products = [
            ["Apple", 1.50, 100],
            ["Banana", 0.75, 200],
            ["Cherry", 2.00, 150],
            ["Date", 3.50, 50]
        ]
        
        # Get product names column
        product_names = [products[0][0], products[1][0], products[2][0], products[3][0]]
        
        # Use MATCH to find position, then INDEX to get value
        banana_pos = match("Banana", product_names, 0)
        banana_price = index(products, banana_pos, 2)
        banana_quantity = index(products, banana_pos, 3)
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let banana_pos = interpreter.get_value("banana_pos").await.unwrap();
    assert_eq!(banana_pos.to_string(), "2");
    
    let banana_price = interpreter.get_value("banana_price").await.unwrap();
    assert_eq!(banana_price.to_string(), "0.75");
    
    let banana_quantity = interpreter.get_value("banana_quantity").await.unwrap();
    assert_eq!(banana_quantity.to_string(), "200");
}

#[tokio::test]
async fn test_vlookup_edge_cases() {
    let mut interpreter = Interpreter::new();
    
    let script = r#"
        # Test with empty array
        empty = []
        result1 = vlookup("test", empty, 1, false)
        
        # Test with single row
        single = [["Apple", 1.50]]
        result2 = vlookup("Apple", single, 2, false)
        
        # Test with column index out of bounds
        data = [["Apple", 1.50], ["Banana", 0.75]]
        # This should error, but we'll catch it
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let result1 = interpreter.get_value("result1").await.unwrap();
    assert_eq!(result1.to_string(), "#N/A");
    
    let result2 = interpreter.get_value("result2").await.unwrap();
    assert_eq!(result2.to_string(), "1.5");
}

#[tokio::test]
async fn test_xlookup_advanced_modes() {
    let mut interpreter = Interpreter::new();
    
    let script = r#"
        # Create test data
        values = [10, 20, 30, 40, 50]
        results = ["A", "B", "C", "D", "E"]
        
        # Test exact match or next smallest (match_mode = -1)
        result1 = xlookup(25, values, results, "None", -1)  # Should return "B" (20)
        
        # Test exact match or next largest (match_mode = 1)
        result2 = xlookup(25, values, results, "None", 1)  # Should return "C" (30)
        
        # Test search from last to first (search_mode = -1)
        duplicate_values = [10, 20, 20, 30]
        duplicate_results = ["First", "Second", "Third", "Fourth"]
        result3 = xlookup(20, duplicate_values, duplicate_results, "None", 0, -1)  # Should return "Third"
    "#;
    
    interpreter.execute(script).await.unwrap();
    
    let result1 = interpreter.get_value("result1").await.unwrap();
    assert_eq!(result1.to_string(), "B");
    
    let result2 = interpreter.get_value("result2").await.unwrap();
    assert_eq!(result2.to_string(), "C");
    
    let result3 = interpreter.get_value("result3").await.unwrap();
    assert_eq!(result3.to_string(), "Third");
}