# Argon-PDF
a simple pdf reader for windows 11 developed in RUST-TAURI.
THis is a  native app that opens a pdf file and displays it for reading. the goal of the app is to allow the user to read a page (and scroll through the pages normally), while also keeping another page viewer open from the same pdf file.
there are three main panels organized vertically:
left panel is the preview of the pages - it is a scrollable window where the user can scroll through the pages and click them to visualize them
the central panel is the page viewer (the user can zoom in-out to read the page)
the third panel on the right is another page viewer that allows the user to access (i.e. view) a second page of the same pdf at the same time (basically it is a duplicate viewer, but with its own scroll bar so that the two viewers are independent).
there is no save/edit of the text. Only highlighting of the text is necessary.
When the user highlights parts of the text (sentences/paragraphs). Every highlighted part of the text will be logged permanently, so that when the user clicks a button `notes` a window is opened (or a panel, we can discuss together the possible options), it can click on the notes and then visualize the page where the note was taken.
The user must be able also to delete (or remove) the highlights

Very important point: you must draft a test strategy so that I can confidently add/remove features from the app during its development, so that I am not afaraid of breaking or introducing any regression. 