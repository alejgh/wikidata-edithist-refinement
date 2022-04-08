FROM jupyter/base-notebook

# Copy the requirements.txt file
COPY requirements.txt requirements.txt

# Install all Python dependencies
RUN python3 -m pip install -r requirements.txt

# install Rust dependencies
USER root
RUN apt-get -qq update && apt-get -qq install -y curl build-essential cmake
RUN usermod --home "/home/alejgh" --login "alejgh" jovyan
RUN mkdir /home/alejgh 
RUN cp -a /home/jovyan/. /home/alejgh
RUN chown ${CHOWN_HOME_OPTS} "alejgh:${NB_GID}" "/home/alejgh"
ENV HOME="/home/alejgh"

USER alejgh
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="${HOME}/.cargo/bin:${PATH}"

RUN cargo install evcxr_jupyter
RUN evcxr_jupyter --install
#RUN cp -a /home/jovyan/. "/home/alejgh/"