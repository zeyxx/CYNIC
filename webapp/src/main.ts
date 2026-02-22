import { LayoutManager } from './ui/layout-manager';
import { ChatClient } from './api/chat';
import { LearningClient } from './api/learning';
import './ui/styles/layout.css';

async function initializeApp(): Promise<void> {
  const layoutManager = new LayoutManager();
  const chatClient = new ChatClient();
  const learningClient = new LearningClient();

  let sessionId = `session-${Date.now()}`;
  let lastJudgmentId = '';
  let lastPrompt = '';
  let lastCode = '';

  // Store learningClient reference to enable future learning event submission (Phase 2)
  void learningClient;

  // Mount layout
  const layoutElement = layoutManager.render();
  document.body.appendChild(layoutElement);

  const chatPanel = layoutManager.getChatPanel();
  const codePanel = layoutManager.getCodePanel();

  // Wire chat send event
  chatPanel.onSend(async (text) => {
    // Add user message to history
    chatPanel.addMessage({
      id: `msg-${Date.now()}`,
      role: 'user',
      content: text,
      judgment_id: '',
      timestamp: Date.now(),
    });

    // Store prompt for learning event
    lastPrompt = text;

    try {
      // Call API
      codePanel.setStatus('executing');
      const response = await chatClient.sendMessage({
        text,
        session_id: sessionId
      });

      // Store judgment_id and code for later feedback
      lastJudgmentId = response.judgment_id;
      lastCode = response.code;

      // Display code in right panel
      codePanel.setCode(response.code, response.language);
      codePanel.setStatus('generated');

      // Log feedback data for Phase 2 learning event submission
      console.log('*sniff* Storing feedback data for learning event', {
        lastPrompt,
        lastCode,
        lastJudgmentId
      });

      // Add assistant message to history
      chatPanel.addMessage({
        id: `msg-${Date.now()}`,
        role: 'assistant',
        content: `Generated ${response.language} code`,
        code: response.code,
        code_language: response.language,
        judgment_id: lastJudgmentId,
        timestamp: Date.now(),
      });

      console.log('*sniff* Chat response received', {
        code: response.code,
        language: response.language,
        judgmentId: lastJudgmentId
      });

    } catch (error) {
      codePanel.setStatus('error');
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      chatPanel.addMessage({
        id: `msg-${Date.now()}`,
        role: 'assistant',
        content: `*GROWL* Error: ${errorMessage}`,
        judgment_id: '',
        timestamp: Date.now(),
      });
      console.error('*GROWL* Chat error:', error);
    }
  });

  console.log('*sniff* CYNIC Chat/Code initialized');
  console.log(`Session ID: ${sessionId}`);
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initializeApp);
} else {
  initializeApp();
}
