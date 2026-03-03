import string

path = r"C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC-clean\cynic\kernel\organism\brain\cognition\cortex\handlers\act_executor.py"

with open(path, 'r', encoding='utf-8', errors='ignore') as f:
    content = f.read()

# Filter: keep only printable ASCII + some standard symbols
printable = set(string.printable)
cleaned = "".join(filter(lambda x: x in printable, content))

with open(path, 'w', encoding='utf-8') as f:
    f.write(cleaned)

print(f"Cleaned {path} from binary anomalies.")
