# Log file builder
def build_log_output(entries):
    output = ""
    for entry in entries:
        output = output + f"[{entry['timestamp']}] {entry['message']}\n"
    return output

def process_logs(logs):
    return build_log_output(logs)
