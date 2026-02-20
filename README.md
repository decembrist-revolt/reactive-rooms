# reactive-chat-rust

Rust-порт проекта [reactive-rooms](https://github.com/decembrist-market/reactive-rooms) на базе Axum + Keycloak.

## Архитектура

- **Web-фреймворк**: Axum
- **Аутентификация**: Keycloak (OIDC/JWT) через `axum-keycloak-auth`
- **Хранилище**: In-memory DashMap (комнаты и участники)
- **Message Bus**: Tokio MPSC-каналы (замена Vert.x Event Bus)
- **WebSocket**: встроенная поддержка Axum

## Роли (OAuth2 scopes)

| Scope | Роль | Доступ |
|---|---|---|
| `reactive-rooms:scope:write` | Admin | Управление комнатами через REST |
| `reactive-rooms:scope:host` | Host | Подключение к комнате как хост |
| `reactive-rooms:scope:user` | User | Подключение к комнате как участник |

## Конфигурация

Скопируй `.env.exampl` в `.env` и заполни:

```env
HOST=0.0.0.0
PORT=3001
ORIGINS=[http://localhost:8080,http://127.0.0.1:8080]
KEYCLOAK_SERVER=https://localhost:8443/
KEYCLOAK_REALM=decembrist-market
KEYCLOAK_AUDIENCE=account
```

### Уровень логирования

```env
RUST_LOG=reactive_chat_rust=info   # только логи приложения
RUST_LOG=debug                      # всё, включая внешние крейты
```

## Запуск

```bash
cp .env.exampl .env
# отредактируй .env
RUST_LOG=info cargo run
```

## Запуск Keycloak (для разработки)

```bash
docker run -p 8080:8080 \
  -e KC_BOOTSTRAP_ADMIN_USERNAME=admin \
  -e KC_BOOTSTRAP_ADMIN_PASSWORD=admin \
  quay.io/keycloak/keycloak:latest \
  start-dev
```

Затем создай realm, клиента и необходимые scopes в Keycloak Admin UI (`http://localhost:8080`).

## API

### Публичные эндпоинты

```
GET /ping     → {"ping": "pong!"}
GET /health   → {"ping": "pong!"}
```

### REST API (требует Bearer токен с ролью Admin)

#### Создать комнату

```
POST /api/rooms
Authorization: Bearer <token>
Content-Type: application/json

{
  "type": "game",
  "hostId": "<userId>"
}

→ 201 Created
{ "roomId": "<uuid>" }
```

#### Получить список комнат

```
GET /api/rooms?page=0&size=10
Authorization: Bearer <token>

→ 200 OK
{
  "rooms": [
    {
      "roomId": "<uuid>",
      "hostId": "<userId>",
      "type": "game",
      "playerCount": 3
    }
  ],
  "totalRooms": 1,
  "page": 0,
  "size": 10
}
```

#### Удалить комнату

```
DELETE /api/rooms/{roomId}
Authorization: Bearer <token>

→ 204 No Content
```

При удалении все подключённые участники получают событие `Disconnect` с причиной `RoomClosed`.

### WebSocket

```
GET /websocket?token=<jwt>&roomId=<uuid>&type=host|user
```

#### Подключение хоста (`type=host`)

Требует роль `Host`. Пользователь должен быть указан как `hostId` при создании комнаты.

#### Подключение участника (`type=user`)

Требует роль `User`.

### WebSocket протокол

#### Сообщения от участника к хосту

```json
{ "event": "MESSAGE", "message": { } }
```

#### Сообщения от хоста к участнику

```json
{ "event": "MESSAGE",    "userId": "<userId>", "message": { } }
{ "event": "DISCONNECT", "userId": "<userId>", "message": { "reason": "Kicked" } }
```

#### Сообщения, которые получает хост

```json
{ "event": "JoinRoom",   "user_id": "<userId>" }
{ "event": "LeaveRoom",  "user_id": "<userId>" }
{ "event": "Message",    "user_id": "<userId>", "message": { } }
{ "event": "Disconnect", "user_id": "<userId>", "message": { "reason": "UserClosed" } }
```

#### Причины отключения (`DisconnectReason`)

| Значение | Описание |
|---|---|
| `Kicked` | Участник выгнан хостом |
| `RoomClosed` | Комната закрыта (хост отключился или DELETE /api/rooms) |
| `UserClosed` | Участник закрыл соединение |
| `NewConnection` | Новое соединение вытеснило старое |
| `PingPong` | Таймаут ping/pong (30 сек интервал, 10 сек на ответ) |
