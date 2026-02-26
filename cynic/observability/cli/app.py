"""CLI Application Framework for CYNIC Observability.

Provides interactive command-line interface for observing and interacting with
the CYNIC + Human + Machine symbiotic system.
"""

from __future__ import annotations

import asyncio


class CliApp:
    """Main CLI application for CYNIC observability.

    Manages the interactive menu structure, user input, and routing to
    different observability views and features.
    """

    def __init__(self) -> None:
        """Initialize CliApp with running flag."""
        self._running = True

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
            ('5', '🤝 SYMBIOSIS    - Alignment'),
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
        print("\n" + "=" * 60)
        print(" CYNIC OBSERVABILITY CLI - Main Menu")
        print("=" * 60)

        menu_items = self.get_menu_items()
        for key, label in menu_items:
            print(f"  [{key}] {label}")

        print("=" * 60)

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
            print("\nGoodbye! 👋")

        elif choice == '1':
            # OBSERVE
            await self.show_observe()

        elif choice == '2':
            # CYNIC MIND
            print("\n💭 CYNIC MIND - Deep Dive")
            print("-" * 40)
            print("[Feature not yet implemented]")

        elif choice == '3':
            # YOUR STATE
            print("\n🧠 YOUR STATE - Energy & Focus")
            print("-" * 40)
            print("[Feature not yet implemented]")

        elif choice == '4':
            # MACHINE
            print("\n⚙️  MACHINE - Resources")
            print("-" * 40)
            print("[Feature not yet implemented]")

        elif choice == '5':
            # SYMBIOSIS
            print("\n🤝 SYMBIOSIS - Alignment")
            print("-" * 40)
            print("[Feature not yet implemented]")

        elif choice == '6':
            # TALK
            print("\n💬 TALK - Chat")
            print("-" * 40)
            print("[Feature not yet implemented]")

        elif choice == '7':
            # HISTORY
            print("\n📊 HISTORY - Decisions")
            print("-" * 40)
            print("[Feature not yet implemented]")

        elif choice == '8':
            # FEEDBACK
            print("\n🎛️  FEEDBACK - Feedback")
            print("-" * 40)
            print("[Feature not yet implemented]")

        elif choice == '9':
            # ACTUATE
            print("\n🚀 ACTUATE - Actions")
            print("-" * 40)
            print("[Feature not yet implemented]")

        else:
            # Invalid choice
            print(f"\n⚠️  Invalid choice: {choice}")
            print("Please enter a number between 0 and 9.")

    async def show_observe(self) -> None:
        """Show quick observation view of all three streams.

        Displays a brief snapshot of CYNIC's thinking, Human's state,
        and Machine's resources in one view.
        """
        print("\n👁️  OBSERVE - Current State Snapshot")
        print("-" * 60)
        print("CYNIC Observations:    [Observing...]")
        print("Human State:           [Monitoring...]")
        print("Machine Resources:     [Scanning...]")
        print("-" * 60)

    async def run(self) -> None:
        """Main CLI loop.

        Continuously displays menu and processes user choices until
        user chooses to exit (sets _running to False).
        """
        print("\n" + "=" * 60)
        print(" Welcome to CYNIC Observability CLI")
        print("=" * 60)
        print("\nStarting interactive menu...")

        try:
            while self._running:
                await self.show_menu()
        except KeyboardInterrupt:
            print("\n\nInterrupted by user. Shutting down...")
            self._running = False
        except EOFError:
            print("\n\nEnd of input. Shutting down...")
            self._running = False


async def main() -> None:
    """Entry point for CLI application."""
    app = CliApp()
    await app.run()


if __name__ == "__main__":
    asyncio.run(main())
