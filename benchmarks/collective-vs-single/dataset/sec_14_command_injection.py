# PDF generation service
import os

def generate_pdf(filename):
    command = f"wkhtmltopdf /tmp/{filename}.html /tmp/{filename}.pdf"
    os.system(command)
    return f"/tmp/{filename}.pdf"

def handle_request(request):
    filename = request.args.get('filename')
    return generate_pdf(filename)
