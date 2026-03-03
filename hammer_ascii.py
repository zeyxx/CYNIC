import os

def final_clean(filepath):
    try:
        with open(filepath, 'rb') as f:
            raw = f.read()
        
        # This is a hammer. We want ONLY clean ASCII for logic and logs.
        # Comments can be stripped of non-ascii.
        
        # Common corrupt sequences in this repo:
        # â€" -> -
        # â’ -> ->
        # â• -> =
        # âœ… -> [OK]
        # âšï¸ -> [WARN]
        # Ã— -> x
        
        replacements = [
            (b'\xe2\x80\x94', b'-'),
            (b'\xc3\xa2\xe2\x82\xac\xe2\x84\xa2', b"'"),
            (b'\xc3\xa2\xe2\x82\xac\x22', b'-'),
            (b'\xc3\xa2\xe2\x82\xac\x9c', b'-'),
            (b'\xc3\xa2\xe2\x82\xac\x9d', b'-'),
            (b'\xc3\xa2\xe2\x80\x9d', b'->'),
            (b'\xc3\xa2\xe2\x80\x99', b'->'),
            (b'\xc3\xa2\xe2\x80\x98', b'->'),
            (b'\xc3\xa2\x27', b'->'),
            (b'\xc3\xa2\xe2\x80\x9c', b'-'),
            (b'\xe2\x80\x9c', b'"'),
            (b'\xe2\x80\x9d', b'"'),
            (b'\xe2\x80\x99', b"'"),
            (b'\xc3\x97', b'x'),
            (b'\xc3\xaf', b'i'),
            (b'\xc3\xaa', b'e'),
            (b'\xc3\xa0', b'a'),
            (b'\xc3\xb4', b'o'),
            # Box drawing and symbols
            (b'\xc3\xa2\xe2\x80\xa2\xe2\x80\x90', b'='),
            (b'\xc3\xa2\xc5\x93\xc2\xae', b'[OK]'),
            (b'\xc3\xa2\xc5\x93\x22', b'[OK]'),
            (b'\xc3\xa2\xc5\x93\xe2\x80\x9c', b'[MATCH]'),
            (b'\xc3\xa2\xc5\x93\x2d', b'[DIVERGE]'),
        ]
        
        new_data = raw
        for old, new in replacements:
            new_data = new_data.replace(old, new)
            
        # Final pass: kill anything remaining that is non-ascii in a string context
        # but keep it if it's UTF-8 valid (though we prefer ASCII for this Windows env)
        try:
            text = new_data.decode('utf-8')
            # Replace remaining common ones textually
            text = text.replace('â€"', '-')
            text = text.replace('â’', '->')
            text = text.replace('â•', '=')
            text = text.replace('âœ…', '[OK]')
            text = text.replace('âš ï¸', '[!!]')
            text = text.replace('âž¡', '->')
            text = text.replace('Ï', 'PHI') # Special case for the golden ratio symbol
            
            # Final safety: remove any char > 127
            clean_text = "".join([c if ord(c) < 128 else '?' for c in text])
            final_data = clean_text.encode('ascii')
        except:
            # If decode fails, just do a byte-level non-ascii strip
            final_data = bytes([b if b < 128 else ord('?') for b in new_data])

        if final_data != raw:
            with open(filepath, 'wb') as f:
                f.write(final_data)
            return True
    except Exception as e:
        print(f"Error: {filepath} -> {e}")
    return False

count = 0
for root, dirs, files in os.walk('cynic'):
    for file in files:
        if file.endswith('.py'):
            if final_clean(os.path.join(root, file)):
                count += 1
print(f"Hammered {count} files into ASCII.")
