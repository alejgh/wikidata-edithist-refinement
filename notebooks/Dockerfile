FROM jupyter/base-notebook

# Copy the requirements.txt file
COPY requirements.txt requirements.txt

# Install all Python dependencies
RUN python3 -m pip install -r requirements.txt

# install Rust
USER root
RUN apt-get -qq update && apt-get -qq install -y curl build-essential cmake

USER ${NB_USER}
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# set up rust jupyter kernel
ENV PATH "~/.cargo/bin:${PATH}"
RUN ~/.cargo/bin/cargo install evcxr_jupyter
RUN ~/.cargo/bin/evcxr_jupyter --install

# The base container takes care of the rest.