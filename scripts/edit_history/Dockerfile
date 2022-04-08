FROM rust:1.58

WORKDIR /code

RUN apt update && apt install -y p7zip-full curl wget

ENTRYPOINT ["scripts/edit_history/run_all.sh"]