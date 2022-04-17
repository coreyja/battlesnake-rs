POST http://localhost:3000/constant-carter/move
Content-Type: application/json
file,fixtures/start_of_game.json;

HTTP/1.1 200

[Asserts]
jsonpath "$.move" == "right"
