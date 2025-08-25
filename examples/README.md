# Agent Core Examples

This directory contains example applications demonstrating how to use the `agent-core` library.

## Available Examples

1. **python_assistant** - Console application with real-time progress display
2. **python_assistant_tui** - Terminal UI with split views and plan tracking (requires `--features tui`)

Both examples load their system prompt from `examples/system_prompt.md` for easy customization.

## Python Assistant Example

A console application that uses agent-core to create an AI-powered Python programming assistant.

### Features

- **Real-time Progress Display**: Shows what the agent is doing with live updates
- **Plan Tracking**: Displays current plan with emoji status indicators (⏳ 🔄 ✅)
- **Tool Execution Visibility**: See when tools are running and their output
- **Error Handling**: Clear error messages with troubleshooting tips
- **Python Environment Setup**: Simplified with `uv run` - no activation needed

### Prerequisites

1. **Install `uv`** (Python package manager):
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```

2. **Set up API Access**:
   ```bash
   export OPENAI_API_KEY="your-api-key-here"
   ```

3. **Ensure Codex Backend is Running** (required for agent-core):
   - The agent-core library requires the Codex backend services
   - See the main README for backend setup instructions

### Running the Example

```bash
cargo run --example python_assistant
```

### Usage Examples

Once the assistant is running, you can ask it to:

- **Calculate Fibonacci numbers**: "Calculate fibonacci numbers up to 100"
- **Data Analysis**: "Download a CSV from URL and create a summary"
- **Visualizations**: "Create a bar chart showing sales data"
- **Math Problems**: "Solve the quadratic equation x^2 - 5x + 6 = 0"
- **File Processing**: "Read all .txt files in /tmp and count words"
- **Web Scraping**: "Get the current weather for San Francisco"

### How It Works

1. **Environment Setup**: `uv` is checked on startup - no manual environment needed
2. **User Input**: You describe what you want to accomplish in natural language
3. **AI Processing**: The agent:
   - Creates a plan to solve your problem
   - Writes a Python script to `/tmp/` with inline dependencies
   - Executes the script using `uv run` (auto-installs dependencies)
   - Returns the results
4. **Output**: You see both the generated code and its execution results

### Example Interaction

```
You: Calculate the first 10 prime numbers

🤖 Assistant:
📝 Plan update:
   - [Pending] Write Python script to find prime numbers
   - [Pending] Execute the script
   - [Pending] Display results

I'll create a Python script to calculate the first 10 prime numbers.

```python
def is_prime(n):
    if n < 2:
        return False
    for i in range(2, int(n**0.5) + 1):
        if n % i == 0:
            return False
    return True

def first_n_primes(n):
    primes = []
    num = 2
    while len(primes) < n:
        if is_prime(num):
            primes.append(num)
        num += 1
    return primes

# Calculate first 10 prime numbers
result = first_n_primes(10)
print("The first 10 prime numbers are:")
print(result)
```

📋 Command output:
The first 10 prime numbers are:
[2, 3, 5, 7, 11, 13, 17, 19, 23, 29]
```

## Architecture

The example demonstrates key features of the `agent-core` library:

1. **Agent Configuration**: Setting up an AI agent with specific capabilities
2. **Tool Integration**: Using bash commands, file operations
3. **Message Channels**: Async communication between UI and agent
4. **Plan Management**: Tracking multi-step task execution
5. **Error Handling**: Graceful error recovery and user feedback

## Customization

You can modify the example to:

- Use different AI models (change `model` in `AgentConfig`)
- Add more tools (file operations, web search, etc.)
- Customize the system prompt for different behaviors
- Add persistence to save conversation history
- Integrate with different programming languages or tools

## Python Assistant TUI

Terminal User Interface version with enhanced visualization and real-time updates.

### Running the TUI

```bash
cargo run --features tui --example python_assistant_tui
```

### TUI Features

- **Split View**: Conversation history, input area, and status bar
- **Real-time Plan Updates**: Live tracking with emoji status indicators
  - ⏳ Pending tasks
  - 🔄 In-progress tasks
  - ✅ Completed tasks
- **Scrollable Messages**: Use ↑↓ arrows to scroll through long conversations
- **Error Display**: Clear error messages with ❌ indicators
- **Status Bar**: Shows current agent status and Python environment state
- **Keyboard Shortcuts**:
  - `Enter`: Send message
  - `↑/↓`: Scroll messages
  - `PageUp/PageDown`: Fast scroll
  - `Ctrl+C` or `Ctrl+Q`: Quit

## Troubleshooting

1. **"uv not found"**: Install uv using the command in Prerequisites
2. **API Key Issues**: Ensure your OpenAI API key is properly set
3. **Backend Connection Failed**: Ensure Codex backend services are running
4. **Permission Errors**: The examples use `/tmp` which should be writable
5. **Package Installation Fails**: Basic Python will still work even if some packages fail to install
6. **Duplicate Output**: Fixed - examples now properly handle streaming vs non-streaming output
7. **TUI Errors Disappearing**: Fixed - errors are now more persistent and visible
8. **System Prompt**: Customize by editing `examples/system_prompt.md`
