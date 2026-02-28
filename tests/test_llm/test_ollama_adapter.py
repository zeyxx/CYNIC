"""
Tests for OllamaAdapter - Local LLM Provider

Tests the OllamaAdapter class for:
- Model initialization
- Availability checking
- Message building (system + user)
- Tool calling support
- Response parsing (dict vs object)
"""
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.brain.llm.adapter import OllamaAdapter, OllamaConnectionPool, LLMRequest, LLMResponse


class TestOllamaAdapter:
    """Test suite for OllamaAdapter."""

    def test_adapter_id_format(self):
        """Adapter ID should be provider:model format."""
        adapter = OllamaAdapter(model="llama3.2")
        assert adapter.adapter_id == "ollama:llama3.2"

    def test_adapter_id_with_custom_model(self):
        """Custom model names should work."""
        adapter = OllamaAdapter(model="mistral:7b")
        assert adapter.adapter_id == "ollama:mistral:7b"

    def test_default_url(self):
        """Default URL should be localhost:11434."""
        adapter = OllamaAdapter()
        assert adapter.base_url == "http://localhost:11434"

    def test_custom_url_stripping(self):
        """Custom URLs should have trailing slash removed."""
        adapter = OllamaAdapter(base_url="http://localhost:11434/")
        assert adapter.base_url == "http://localhost:11434"

    @pytest.mark.asyncio
    async def test_complete_with_prompt(self):
        """Should handle simple prompt completion."""
        adapter = OllamaAdapter(model="llama3.2")
        
        # Mock the ollama client
        mock_response = MagicMock()
        mock_response.message.content = "Test response"
        mock_response.prompt_eval_count = 10
        mock_response.eval_count = 20
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.chat = AsyncMock(return_value=mock_response)
            mock_get_client.return_value = mock_client
            
            request = LLMRequest(prompt="Hello")
            response = await adapter.complete(request)
            
            assert response.content == "Test response"
            assert response.model == "llama3.2"
            assert response.provider == "ollama"
            assert response.prompt_tokens == 10
            assert response.completion_tokens == 20

    @pytest.mark.asyncio
    async def test_complete_with_system_message(self):
        """Should prepend system message to messages."""
        adapter = OllamaAdapter(model="llama3.2")
        
        mock_response = MagicMock()
        mock_response.message.content = "Response"
        mock_response.prompt_eval_count = 5
        mock_response.eval_count = 10
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.chat = AsyncMock(return_value=mock_response)
            mock_get_client.return_value = mock_client
            
            request = LLMRequest(prompt="Hello", system="You are helpful")
            await adapter.complete(request)
            
            # Check that system was prepended
            call_args = mock_client.chat.call_args
            messages = call_args.kwargs.get('messages', [])
            assert messages[0]['role'] == 'system'
            assert messages[0]['content'] == 'You are helpful'

    @pytest.mark.asyncio
    async def test_complete_with_messages_override(self):
        """Messages should override prompt if provided."""
        adapter = OllamaAdapter(model="llama3.2")
        
        mock_response = MagicMock()
        mock_response.message.content = "Response"
        mock_response.prompt_eval_count = 5
        mock_response.eval_count = 10
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.chat = AsyncMock(return_value=mock_response)
            mock_get_client.return_value = mock_client
            
            messages = [
                {"role": "user", "content": "Previous message"}
            ]
            request = LLMRequest(prompt="Ignore this", messages=messages)
            await adapter.complete(request)
            
            call_args = mock_client.chat.call_args
            messages = call_args.kwargs.get('messages', [])
            assert len(messages) == 1
            assert messages[0]['content'] == "Previous message"

    @pytest.mark.asyncio
    async def test_complete_with_tools(self):
        """Should pass tools to the model."""
        adapter = OllamaAdapter(model="llama3.2")
        
        mock_response = MagicMock()
        mock_response.message.content = "Response"
        mock_response.prompt_eval_count = 5
        mock_response.eval_count = 10
        
        tools = [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather",
                    "parameters": {"type": "object", "properties": {}}
                }
            }
        ]
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.chat = AsyncMock(return_value=mock_response)
            mock_get_client.return_value = mock_client
            
            request = LLMRequest(prompt="Hello", tools=tools)
            await adapter.complete(request)
            
            call_args = mock_client.chat.call_args
            assert 'tools' in call_args.kwargs

    @pytest.mark.asyncio
    async def test_check_available_success(self):
        """Should return True when Ollama is available."""
        adapter = OllamaAdapter()
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.list = AsyncMock(return_value={"models": []})
            mock_get_client.return_value = mock_client
            
            result = await adapter.check_available()
            assert result is True

    @pytest.mark.asyncio
    async def test_check_available_failure(self):
        """Should return False when Ollama is unavailable."""
        import httpx
        adapter = OllamaAdapter()
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.list = AsyncMock(side_effect=httpx.RequestError("Connection failed"))
            mock_get_client.return_value = mock_client
            
            result = await adapter.check_available()
            assert result is False

    @pytest.mark.asyncio
    async def test_list_models(self):
        """Should return list of model names."""
        adapter = OllamaAdapter()
        
        mock_response = {
            "models": [
                {"name": "llama3.2"},
                {"name": "mistral:7b"},
                {"name": "gemma2:2b"}
            ]
        }
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.list = AsyncMock(return_value=mock_response)
            mock_get_client.return_value = mock_client
            
            models = await adapter.list_models()
            
            assert "llama3.2" in models
            assert "mistral:7b" in models
            assert "gemma2:2b" in models

    @pytest.mark.asyncio
    async def test_response_dict_format(self):
        """Should handle dict response format (ollama 0.1.x)."""
        adapter = OllamaAdapter(model="llama3.2")
        
        # Dict format (older ollama)
        mock_response = {
            "message": {
                "content": "Dict response"
            },
            "prompt_eval_count": 8,
            "eval_count": 15
        }
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.chat = AsyncMock(return_value=mock_response)
            mock_get_client.return_value = mock_client
            
            request = LLMRequest(prompt="Test")
            response = await adapter.complete(request)
            
            assert response.content == "Dict response"
            assert response.prompt_tokens == 8
            assert response.completion_tokens == 15

    @pytest.mark.asyncio
    async def test_tool_calls_parsing(self):
        """Should parse tool calls from response."""
        adapter = OllamaAdapter(model="llama3.2")
        
        mock_response = MagicMock()
        mock_response.message.content = "I'll check the weather"
        mock_response.message.tool_calls = [
            {
                "function": {
                    "name": "get_weather",
                    "arguments": {"location": "Paris"}
                }
            }
        ]
        mock_response.prompt_eval_count = 10
        mock_response.eval_count = 20
        
        with patch.object(adapter.pool, 'get_client') as mock_get_client:
            mock_client = AsyncMock()
            mock_client.chat = AsyncMock(return_value=mock_response)
            mock_get_client.return_value = mock_client
            
            request = LLMRequest(prompt="What's the weather?")
            response = await adapter.complete(request)
            
            assert response.tool_calls is not None
            assert len(response.tool_calls) == 1
            assert response.tool_calls[0]["name"] == "get_weather"


class TestOllamaConnectionPool:
    """Test suite for OllamaConnectionPool."""

    def test_pool_initialization(self):
        """Pool should initialize empty."""
        pool = OllamaConnectionPool()
        assert pool._clients == {}

    def test_get_client_creates_new(self):
        """Should create new client for new base_url."""
        pool = OllamaConnectionPool()
        
        with patch('cynic.brain.llm.adapter.ollama') as mock_ollama:
            client = pool.get_client("http://localhost:11434")
            
            assert "http://localhost:11434" in pool._clients
            mock_ollama.AsyncClient.assert_called_once()

    def test_get_client_reuses_existing(self):
        """Should reuse existing client for same base_url."""
        pool = OllamaConnectionPool()
        
        with patch('cynic.brain.llm.adapter.ollama') as mock_ollama:
            client1 = pool.get_client("http://localhost:11434")
            client2 = pool.get_client("http://localhost:11434")
            
            assert client1 is client2
            assert mock_ollama.AsyncClient.call_count == 1

    @pytest.mark.asyncio
    async def test_close_all(self):
        """Should close all cached clients."""
        pool = OllamaConnectionPool()
        
        mock_client1 = AsyncMock()
        mock_client2 = AsyncMock()
        pool._clients = {
            "url1": mock_client1,
            "url2": mock_client2
        }
        
        await pool.close_all()
        
        mock_client1.close.assert_called_once()
        mock_client2.close.assert_called_once()
        assert pool._clients == {}


class TestLLMRequest:
    """Test suite for LLMRequest dataclass."""

    def test_default_values(self):
        """Should have correct defaults."""
        request = LLMRequest(prompt="test")
        
        assert request.prompt == "test"
        assert request.system == ""
        assert request.max_tokens == 2048
        assert request.temperature == 0.0
        assert request.stream is False

    def test_custom_values(self):
        """Should accept custom values."""
        request = LLMRequest(
            prompt="test",
            system="You are AI",
            max_tokens=1024,
            temperature=0.7,
            stream=True
        )
        
        assert request.prompt == "test"
        assert request.system == "You are AI"
        assert request.max_tokens == 1024
        assert request.temperature == 0.7
        assert request.stream is True


class TestLLMResponse:
    """Test suite for LLMResponse dataclass."""

    def test_total_tokens(self):
        """Should compute total tokens correctly."""
        response = LLMResponse(
            content="test",
            model="llama3.2",
            provider="ollama",
            prompt_tokens=100,
            completion_tokens=50
        )
        
        assert response.total_tokens == 150

    def test_is_success(self):
        """Should identify successful responses."""
        success_response = LLMResponse(
            content="test",
            model="llama3.2",
            provider="ollama"
        )
        assert success_response.is_success is True

    def test_is_success_with_error(self):
        """Should identify failed responses."""
        error_response = LLMResponse(
            content="",
            model="llama3.2",
            provider="ollama",
            error="Timeout"
        )
        assert error_response.is_success is False

    def test_tokens_per_second(self):
        """Should compute tokens per second."""
        response = LLMResponse(
            content="test",
            model="llama3.2",
            provider="ollama",
            completion_tokens=100,
            latency_ms=1000  # 1 second
        )
        
        assert response.tokens_per_second == 100.0

    def test_tokens_per_second_zero_latency(self):
        """Should handle zero latency."""
        response = LLMResponse(
            content="",
            model="llama3.2",
            provider="ollama",
            completion_tokens=100,
            latency_ms=0
        )
        
        assert response.tokens_per_second == 0.0
