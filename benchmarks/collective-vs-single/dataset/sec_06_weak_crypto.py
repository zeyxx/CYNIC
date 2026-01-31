# User registration
import hashlib

def hash_password(password):
    return hashlib.md5(password.encode()).hexdigest()

def register_user(username, password):
    hashed = hash_password(password)
    # Store in database
    return {"username": username, "password_hash": hashed}
