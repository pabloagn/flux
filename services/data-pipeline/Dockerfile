FROM python:3.11-slim AS builder

WORKDIR /app

# Copy dependency files first
COPY pyproject.toml .

# Install uv (assumed to be a Python package or installable via pip)
RUN pip install uv

# Run uv to generate/sync lockfile inside the container
RUN uv sync

# Now copy source code
COPY ./src ./src

# Final image stage
FROM python:3.11-slim

WORKDIR /app

# Copy everything from builder
COPY --from=builder /app /app

# Run the app
CMD ["python", "-m", "src.main"]

