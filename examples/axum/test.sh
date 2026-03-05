#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Base URL
BASE_URL="http://127.0.0.1:3000"

# Function to print section headers
print_section() {
    echo -e "\n${BLUE}=== $1 ===${NC}\n"
}

# Function to print success messages
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print error messages
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to print info messages
print_info() {
    echo -e "${YELLOW}ℹ $1${NC}"
}

# Check if server is running
check_server() {
    print_section "Checking Server Status"
    
    if ! curl -s "$BASE_URL/health" > /dev/null; then
        print_error "Server is not running on $BASE_URL"
        exit 1
    fi
    
    print_success "Server is running"
}

# Test Health Check
test_health() {
    print_section "Testing Health Check"
    
    response=$(curl -s -w "\n%{http_code}" "$BASE_URL/health")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "GET /health - Status: $http_code"
        echo "Response: $body"
    else
        print_error "GET /health - Status: $http_code"
    fi
}

# Create Users
test_create_users() {
    print_section "Creating Users"
    
    # User 1
    response=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/users" \
        -H "Content-Type: application/json" \
        -d '{"name":"Alice","email":"alice@example.com"}')
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "201" ]; then
        print_success "POST /users - Created Alice - Status: $http_code"
        USER_ID_1=$(echo "$body" | grep -o '"id":[0-9]*' | head -1 | grep -o '[0-9]*')
        echo "Response: $body"
        echo "Extracted User ID: $USER_ID_1"
    else
        print_error "POST /users - Failed - Status: $http_code"
        return 1
    fi
    
    # User 2
    response=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/users" \
        -H "Content-Type: application/json" \
        -d '{"name":"Bob","email":"bob@example.com"}')
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "201" ]; then
        print_success "POST /users - Created Bob - Status: $http_code"
        USER_ID_2=$(echo "$body" | grep -o '"id":[0-9]*' | head -1 | grep -o '[0-9]*')
        echo "Response: $body"
        echo "Extracted User ID: $USER_ID_2"
    else
        print_error "POST /users - Failed - Status: $http_code"
        return 1
    fi
}

# Get All Users
test_get_all_users() {
    print_section "Getting All Users"
    
    response=$(curl -s -w "\n%{http_code}" "$BASE_URL/users")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "GET /users - Status: $http_code"
        echo "Response: $body"
    else
        print_error "GET /users - Status: $http_code"
    fi
}

# Get User by ID
test_get_user_by_id() {
    print_section "Getting User by ID"
    
    if [ -z "$USER_ID_1" ]; then
        print_error "USER_ID_1 not set, skipping test"
        return 1
    fi
    
    response=$(curl -s -w "\n%{http_code}" "$BASE_URL/users/$USER_ID_1")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "GET /users/$USER_ID_1 - Status: $http_code"
        echo "Response: $body"
    else
        print_error "GET /users/$USER_ID_1 - Status: $http_code"
    fi
}

# Create Todos
test_create_todos() {
    print_section "Creating Todos"
    
    if [ -z "$USER_ID_1" ] || [ -z "$USER_ID_2" ]; then
        print_error "User IDs not set, skipping test"
        return 1
    fi
    
    # Todo 1
    response=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/todos" \
        -H "Content-Type: application/json" \
        -d "{\"user_id\":$USER_ID_1,\"title\":\"Buy groceries\",\"description\":\"Milk, eggs, bread\"}")
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "201" ]; then
        print_success "POST /todos - Created Todo 1 - Status: $http_code"
        TODO_ID_1=$(echo "$body" | grep -o '"id":[0-9]*' | head -1 | grep -o '[0-9]*')
        echo "Response: $body"
        echo "Extracted Todo ID: $TODO_ID_1"
    else
        print_error "POST /todos - Failed - Status: $http_code"
        return 1
    fi
    
    # Todo 2
    response=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/todos" \
        -H "Content-Type: application/json" \
        -d "{\"user_id\":$USER_ID_2,\"title\":\"Write documentation\",\"description\":\"API documentation\"}")
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "201" ]; then
        print_success "POST /todos - Created Todo 2 - Status: $http_code"
        TODO_ID_2=$(echo "$body" | grep -o '"id":[0-9]*' | head -1 | grep -o '[0-9]*')
        echo "Response: $body"
        echo "Extracted Todo ID: $TODO_ID_2"
    else
        print_error "POST /todos - Failed - Status: $http_code"
        return 1
    fi
}

# Get All Todos
test_get_all_todos() {
    print_section "Getting All Todos"
    
    response=$(curl -s -w "\n%{http_code}" "$BASE_URL/todos")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "GET /todos - Status: $http_code"
        echo "Response: $body"
    else
        print_error "GET /todos - Status: $http_code"
    fi
}

# Update Todo Status
test_update_todo_status() {
    print_section "Updating Todo Status"
    
    if [ -z "$TODO_ID_1" ]; then
        print_error "TODO_ID_1 not set, skipping test"
        return 1
    fi
    
    response=$(curl -s -w "\n%{http_code}" -X PUT "$BASE_URL/todos/$TODO_ID_1/status" \
        -H "Content-Type: application/json" \
        -d '{"completed":true}')
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "PUT /todos/$TODO_ID_1/status - Status: $http_code"
        echo "Response: $body"
    else
        print_error "PUT /todos/$TODO_ID_1/status - Status: $http_code"
    fi
}

# Delete Todo
test_delete_todo() {
    print_section "Deleting Todo"
    
    if [ -z "$TODO_ID_2" ]; then
        print_error "TODO_ID_2 not set, skipping test"
        return 1
    fi
    
    response=$(curl -s -w "\n%{http_code}" -X DELETE "$BASE_URL/todos/$TODO_ID_2")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "DELETE /todos/$TODO_ID_2 - Status: $http_code"
        echo "Response: $body"
    else
        print_error "DELETE /todos/$TODO_ID_2 - Status: $http_code"
    fi
}

# Delete User
test_delete_user() {
    print_section "Deleting User"
    
    if [ -z "$USER_ID_2" ]; then
        print_error "USER_ID_2 not set, skipping test"
        return 1
    fi
    
    response=$(curl -s -w "\n%{http_code}" -X DELETE "$BASE_URL/users/$USER_ID_2")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "DELETE /users/$USER_ID_2 - Status: $http_code"
        echo "Response: $body"
    else
        print_error "DELETE /users/$USER_ID_2 - Status: $http_code"
    fi
}

# Get All Users After Deletion
test_get_all_users_final() {
    print_section "Getting All Users (After Deletion)"
    
    response=$(curl -s -w "\n%{http_code}" "$BASE_URL/users")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "GET /users - Status: $http_code"
        echo "Response: $body"
    else
        print_error "GET /users - Status: $http_code"
    fi
}

# Get All Todos After Deletion
test_get_all_todos_final() {
    print_section "Getting All Todos (After Deletion)"
    
    response=$(curl -s -w "\n%{http_code}" "$BASE_URL/todos")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "200" ]; then
        print_success "GET /todos - Status: $http_code"
        echo "Response: $body"
    else
        print_error "GET /todos - Status: $http_code"
    fi
}

# Main execution
main() {
    echo -e "${YELLOW}"
    cat << "EOF"
    ╔═══════════════════════════════════════╗
    ║   FLUXDI Axum API Test Suite            ║
    ║   Testing Dependency Injection        ║
    ║   with RESTful Endpoints              ║
    ╚═══════════════════════════════════════╝
EOF
    echo -e "${NC}"
    
    check_server
    test_health
    test_create_users
    test_get_all_users
    test_get_user_by_id
    test_create_todos
    test_get_all_todos
    test_update_todo_status
    test_delete_todo
    test_delete_user
    test_get_all_users_final
    test_get_all_todos_final
    
    print_section "Test Suite Completed"
    print_success "All tests executed successfully!"
}

# Run main function
main

