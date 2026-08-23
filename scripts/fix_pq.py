with open('samardb/src/pq_harness.lpp', 'r') as f:
    text = f.read()
text = text.replace(chr(39), chr(34))
with open('samardb/src/pq_harness.lpp', 'w') as f:
    f.write(text)
print('Fixed!')
