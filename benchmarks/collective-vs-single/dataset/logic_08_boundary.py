# Statistics calculator
def calculate_average(numbers):
    total = sum(numbers)
    count = len(numbers)
    return total / count  # ZeroDivisionError if empty

def get_stats(data):
    return {
        "average": calculate_average(data),
        "min": min(data),
        "max": max(data)
    }
