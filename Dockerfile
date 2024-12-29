FROM node:latest
RUN apt-get update && apt-get install -y build-essential
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH=$HOME/.cargo/bin:$PATH
CMD bash
