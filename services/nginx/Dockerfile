FROM nginx:alpine

# Both configs ship; entrypoint.sh picks one based on whether origin TLS
# material is mounted at /etc/nginx/tls (see README.md, finding C-1).
COPY nginx-tls.conf   /etc/nginx/available/nginx-tls.conf
COPY nginx-plain.conf /etc/nginx/available/nginx-plain.conf
COPY entrypoint.sh    /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh && mkdir -p /etc/nginx/tls

EXPOSE 80 443
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
