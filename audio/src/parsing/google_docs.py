import os.path
import argparse
import os
from google.auth.transport.requests import Request
from google.oauth2.credentials import Credentials
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build
from googleapiclient.errors import HttpError

# If modifying these scopes, delete the file token.json.
SCOPES = ["https://www.googleapis.com/auth/documents"]

# The ID of a sample document.
DOCUMENT_ID = os.environ.get("DOCUMENT_ID")


def main(text):
  """Shows basic usage of the Docs API.
  Prints the title of a sample document.
  """
  creds = None
  # The file token.json stores the user's access and refresh tokens, and is
  # created automatically when the authorization flow completes for the first
  # time.
  if os.path.exists("Users/j-supha/Desktop/secure_1.json"):
    creds = Credentials.from_authorized_user_file("/Users/j-supha/Desktop/secure_1.json", SCOPES)
  # If there are no (valid) credentials available, let the user log in.
  if not creds or not creds.valid:
    if creds and creds.expired and creds.refresh_token:
      creds.refresh(Request())
    else:
      flow = InstalledAppFlow.from_client_secrets_file(
          "/Users/j-supha/Desktop/Google_Crap/secure_1.json", SCOPES
      )
      creds = flow.run_local_server(port=0)
    # Save the credentials for the next run
    with open("/Users/j-supha/Desktop/Google_Crap/secure.json", "w") as token:
      token.write(creds.to_json())

  try:

    print("Serving to build out the model")
    service = build("docs", "v1", credentials=creds)

    # Retrieve the documents contents from the Docs service.
    document = service.documents().get(documentId=DOCUMENT_ID).execute()

    service.documents().batchUpdate(
      documentId=DOCUMENT_ID,
      body={
        "requests": [
          {
            "insertText": {
              "location": {
                "index": 1,
              },
              "text": f"\n\n{text}\n\n\n",
            },
          },
        ],
      },
    ).execute()
    print("Text successfully written to the document the text is: ", text)
    print(f"The title of the document is: {document.get('title')}")
  except HttpError as err:
    print(err)


if __name__ == "__main__":
  parser = argparse.ArgumentParser(description="Write text to a Google Document.")
  parser.add_argument("--write", type=str, help="Text to write to the document.", default="")
  args = parser.parse_args()
  main(args.write)