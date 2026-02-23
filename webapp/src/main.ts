import { LayoutManager } from './ui/layout-manager';
import { ChatClient } from './api/chat';
import { LearningClient } from './api/learning';
import './ui/styles/layout.css';

export async function initializeApp(): Promise<void> {
  const layoutManager = new LayoutManager();
  const chatClient = new ChatClient();
  const learningClient = new LearningClient(); // Phase 2: used for feedback submission

  let sessionId = `session-${Date.now()}`;
  let lastJudgmentId = '';
  let lastPrompt = '';
  let lastCode = '';

  // Mount layout
  const layoutElement = layoutManager.render();
  document.body.appendChild(layoutElement);

  const chatPanel = layoutManager.getChatPanel();
  const codePanel = layoutManager.getCodePanel();

  // Wire chat send event
  chatPanel.onSend(async (text) => {
    // Add user message to history
    const userTimestamp = Date.now();
    chatPanel.addMessage({
      id: `msg-${userTimestamp}-${Math.random().toString(36).substr(2, 9)}`,
      role: 'user',
      content: text,
      judgment_id: '',
      timestamp: userTimestamp,
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
      const assistantTimestamp = Date.now();
      chatPanel.addMessage({
        id: `msg-${assistantTimestamp}-${Math.random().toString(36).substr(2, 9)}`,
        role: 'assistant',
        content: `Generated ${response.language} code`,
        code: response.code,
        code_language: response.language,
        judgment_id: lastJudgmentId,
        timestamp: assistantTimestamp,
      });

      console.log('*sniff* Chat response received', {
        code: response.code,
        language: response.language,
        judgmentId: lastJudgmentId
      });

    } catch (error) {
      codePanel.setStatus('error');
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      const errorTimestamp = Date.now();
      chatPanel.addMessage({
        id: `msg-${errorTimestamp}-${Math.random().toString(36).substr(2, 9)}`,
        role: 'assistant',
        content: `*GROWL* Error: ${errorMessage}`,
        judgment_id: '',
        timestamp: errorTimestamp,
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
