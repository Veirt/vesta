# https://github.com/nodejs/docker-node/issues/1912
FROM node:20.2.0-alpine3.18 AS builder

WORKDIR /app
COPY package.json yarn.lock ./
RUN yarn install --frozen-lockfile 
COPY . .
RUN yarn build && yarn install --production


ENV NODE_ENV=production
FROM node:20.2.0-alpine3.18
WORKDIR /app
COPY --from=builder /app/build build/
COPY --from=builder /app/node_modules node_modules/
COPY package.json .
CMD ["node", "build"]
