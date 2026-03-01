"""CLI Application Framework for CYNIC Observability.

Provides interactive command-line interface for observing and interacting with
the CYNIC + Human + Machine symbiotic system.
"""

from __future__ import annotations

import asyncio

from cynic.interfaces.cli.dialogue_mode import DialogueMode
from cynic.kernel.observability.cli.views import (
    render_cynic_view,
    render_machine_view,
    render_observe_view,
)
from cynic.kernel.observability.symbiotic_state_manager import get_current_state


class CliApp:
    """Main CLI application for CYNIC observability.

    Manages the interactive menu structure, user input, and routing to
    different observability views and features.
    """

    def __init__(self) -> None:
        """Initialize CliApp with running flag."""
        self._running = True
        self._organism = None

    async def _ensure_organism(self) -> Any:
        """Ensure local organism is awakened and started."""
        if self._organism is None:
            from cynic.kernel.organism.organism import awaken
            self._organism = awaken()
            await self._organism.start()
            
            # Connect to symbiotic state manager
            from cynic.kernel.observability.symbiotic_state_manager import (
                get_symbiotic_state_manager,
            )
            mgr = await get_symbiotic_state_manager()
            mgr.set_organism(self._organism)
            
        return self._organism

    def get_menu_items(self) -> list[tuple[str, str]]:
        """Get menu items as list of (key, label) tuples.

        Returns:
            List of menu items where each item is a tuple of (key, label).
            Keys are string numbers or characters (e.g., '1', '2', '0').
            Labels are descriptive text for the menu option.
        """
        return [
            ('1', '👁️  OBSERVE     - Watch all three streams'),
            ('2', '💭 CYNIC MIND   - Deep dive into CYNIC'),
            ('3', '🧠 YOUR STATE   - Your energy, focus'),
            ('4', '⚙️  MACHINE      - Resources'),
            ('B', '🦴 BODY         - Live Embodied TUI (NEW)'),
            ('5', '🤝 FEDERATION   - P2P gossip status'),
            ('6', '💬 TALK         - Chat'),
            ('7', '📊 HISTORY      - Decisions'),
            ('8', '🎛️  FEEDBACK     - Feedback'),
            ('9', '🚀 ACTUATE      - Actions'),
            ('0', 'EXIT'),
        ]

    async def show_menu(self) -> None:
        """Display menu items and get user choice.

        Prints the menu structure to stdout and prompts for user input.
        """

        menu_items = self.get_menu_items()
        for _key, _label in menu_items:
            pass


        choice = input("\nEnter choice: ").strip()
        await self.handle_menu_choice(choice)

    async def handle_menu_choice(self, choice: str) -> None:
        """Handle user menu choice and route to appropriate handler.

        Args:
            choice: The menu choice (e.g., '1', '2', '0' for exit).
        """
        choice = choice.strip()

        if choice == '0':
            # Exit
            self._running = False

        elif choice == '1':
            # OBSERVE
            await self.show_observe()

        elif choice == '2':
            # CYNIC MIND
            state = await get_current_state()
            render_cynic_view(state)

        elif choice == '3':
            # YOUR STATE
            pass

        elif choice == '4':
            # MACHINE
            state = await get_current_state()
            render_machine_view(state)

        elif choice.upper() == 'B':
            # LIVE BODY TUI
            await self.show_body_tui()

        elif choice == '5':
            # FEDERATION
            await self.show_federation_status()

        elif choice == '6':
            # TALK
            await self.handle_talk_option()

        elif choice == '7':
            # HISTORY
            pass

        elif choice == '8':
            # FEEDBACK
            pass

        elif choice == '9':
            # ACTUATE
            pass

        else:
            # Invalid choice
            pass

    async def show_observe(self) -> None:
        """Show quick observation view of all three streams.

        Displays a brief snapshot of CYNIC's thinking, Human's state,
        and Machine's resources in one view.
        """
        state = await get_current_state()
        render_observe_view(state)

    async def show_body_tui(self) -> None:
        """Launch the live Embodied TUI."""
        
        organism = await self._ensure_organism()
        
        from cynic.interfaces.cli.organism_tui import OrganismTUI
        tui = OrganismTUI(organism)
        
        try:
            await tui.run()
        except KeyboardInterrupt:
            pass

    async def show_federation_status(self) -> None:
        """Show P2P gossip federation status.

        Displays GossipManager stats including connected peers,
        sync count, merged Q-Table keys, and gossip batch size.
        """

        state = await get_current_state()
        organism = state.organism if hasattr(state, 'organism') else state

        if not hasattr(organism, 'gossip_manager') or organism.gossip_manager is None:
            return

        organism.gossip_manager.get_stats()

    async def handle_talk_option(self) -> None:
        """Enter interactive dialogue mode with CYNIC.

        Starts a conversation session where the user can ask questions,
        provide feedback, and explore CYNIC's reasoning in real-time.
        """

        dialogue_mode = DialogueMode()

        try:
            # Initialize dialogue mode
            await dialogue_mode.initialize()

            # Show greeting
            await dialogue_mode.get_greeting()

            # Main dialogue loop
            while True:
                try:
                    user_input = input("You: ").strip()
                except EOFError:
                    # Handle end of input gracefully
                    break

                # Check for exit commands
                if not user_input:
                    continue
                if user_input.lower() in ["back", "exit", "quit", "bye"]:
                    break

                # Process message and get response
                try:
                    await dialogue_mode.process_message(user_input)
                except Exception:
                    continue

        except KeyboardInterrupt:
            pass
        except Exception:
            pass
        finally:
            await dialogue_mode.close()

    async def show_history(self) -> None:
        """Show conversation history.

        Displays past conversations with CYNIC, organized by timestamp.
        """

    async def show_feedback(self) -> None:
        """Show feedback management interface.

        Allows users to review learning metrics, manage Q-Table entries,
        and provide corrective feedback.
        """

    async def run(self) -> None:
        """Main CLI loop.

        Continuously displays menu and processes user choices until
        user chooses to exit (sets _running to False).
        """

        try:
            while self._running:
                await self.show_menu()
        except KeyboardInterrupt:
            self._running = False
        except EOFError:
            self._running = False
        finally:
            if self._organism:
                await self._organism.stop()


async def main() -> None:
    """Entry point for CLI application."""
    app = CliApp()
    await app.run()


if __name__ == "__main__":
    asyncio.run(main())
