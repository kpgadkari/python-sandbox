FROM node:20-bookworm AS build
WORKDIR /app
COPY package.json package.json
RUN npm install
COPY index.html index.html
COPY tsconfig.json tsconfig.json
COPY vite.config.ts vite.config.ts
COPY scripts scripts
COPY src src
RUN npm run build

FROM nginx:1.27-alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
