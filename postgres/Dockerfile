FROM postgres:13.2

ENV LANG C.UTF-8

COPY ./init/setup-primary.sh /docker-entrypoint-initdb.d/setup-primary.sh
COPY ./init/init.sql /docker-entrypoint-initdb.d/init.sql
RUN chmod 0666 /docker-entrypoint-initdb.d/setup-primary.sh