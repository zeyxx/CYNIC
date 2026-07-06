
import threading
import time
from .sensor_base import JarvisSensor

class InputSensor(JarvisSensor):
    def __init__(self):
        super().__init__("pc_input_1", "keyboard_mouse_telemetry")
        self.cps_counter = 0
        self.wpm_counter = 0
        self.last_flush = time.time()
        self.listener_thread = None

    def start(self, emit_callback):
        self.running = True
        self.emit_callback = emit_callback
        
        try:
            from pynput import mouse, keyboard
        except ImportError:
            print("WARNING: pynput not installed. Input sensor running in mock mode.")
            self.listener_thread = threading.Thread(target=self._mock_loop, daemon=True)
            self.listener_thread.start()
            return

        def on_click(x, y, button, pressed):
            if pressed: self.cps_counter += 1

        def on_press(key):
            self.wpm_counter += 1

        self.mouse_listener = mouse.Listener(on_click=on_click)
        self.keyboard_listener = keyboard.Listener(on_press=on_press)
        
        self.mouse_listener.start()
        self.keyboard_listener.start()

        self.listener_thread = threading.Thread(target=self._flush_loop, daemon=True)
        self.listener_thread.start()

    def _mock_loop(self):
        while self.running:
            time.sleep(5)
            self.emit_callback(self.create_signal({"wpm_estimate": 42, "cps_estimate": 1.2}))

    def _flush_loop(self):
        while self.running:
            time.sleep(5)
            now = time.time()
            elapsed = now - self.last_flush
            
            wpm = (self.wpm_counter / 5.0) * 12  # approx words based on chars/5sec * 12
            cps = self.cps_counter / 5.0
            
            if wpm > 0 or cps > 0:
                signal = self.create_signal({
                    "wpm_estimate": round(wpm, 1),
                    "cps_estimate": round(cps, 1),
                    "cognitive_load_heuristic": "high" if wpm > 60 else "normal"
                })
                self.emit_callback(signal)
            
            self.cps_counter = 0
            self.wpm_counter = 0
            self.last_flush = now

    def stop(self):
        self.running = False
        if hasattr(self, "mouse_listener"):
            self.mouse_listener.stop()
            self.keyboard_listener.stop()
