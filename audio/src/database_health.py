import sqlite3

# Connect to the database
conn = sqlite3.connect('audio_text.db')
cursor = conn.cursor()

# Get all table names
cursor.execute("SELECT name FROM sqlite_master WHERE type='table';")
tables = cursor.fetchall()

print("Tables in the database:")
for table in tables:
    print(table[0])
    
    # Get the schema for each table
    cursor.execute(f"PRAGMA table_info({table[0]})")
    columns = cursor.fetchall()
    print("Columns:")
    for column in columns:
        print(f"  {column[1]} ({column[2]})")
    
    # Display a few rows from each table
    cursor.execute(f"SELECT DISTINCT(audio) FROM {table[0]}")
    rows = cursor.fetchall()
    print("Sample data:")
    for row in rows:
        print(row)
    
    print("\n")

# Close the connection
conn.close()


