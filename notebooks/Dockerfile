FROM jupyter/base-notebook

# Copy the requirements.txt file
COPY requirements.txt requirements.txt

# Install all Python dependencies
RUN python3 -m pip install -r requirements.txt

# The base container takes care of the rest.