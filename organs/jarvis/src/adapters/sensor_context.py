
import threading
import time
from .sensor_base import JarvisSensor

class ContextSensor(JarvisSensor):
    def __init__(self):
        super().__init__("pc_wm_1", "desktop_context")
        self.thread = None
        self.last_window = ""

    def start(self, emit_callback):
        self.running = True
        self.emit_callback = emit_callback
        self.thread = threading.Thread(target=self._loop, daemon=True)
        self.thread.start()

    def _get_active_window(self):
        # Stub for xdotool or wayland equivalent
        return "VSCode - CYNIC"

    def _loop(self):
        while self.running:
            time.sleep(2)
            current_window = self._get_active_window()
            if current_window != self.last_window:
                self.emit_callback(self.create_signal({
                    "event": "focus_change",
                    "active_window": current_window
                }))
                self.last_window = current_window

    def stop(self):
        self.running = False
