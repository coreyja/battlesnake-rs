GET http://localhost:3000/constant-carter

HTTP/1.1 200

[Asserts]
jsonpath "$.author" == "coreyja"
jsonpath "$.color" == "#AA66CC"
jsonpath "$.head" == "trans-rights-scarf"
