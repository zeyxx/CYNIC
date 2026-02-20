docker logs cynic-kernel 2>&1 | Select-String "schema|q_table|DB |WARN|ERROR|Traceback|initialized|warm" | Select-Object -First 20
