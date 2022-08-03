# CrossPlay

CrossPlay is a YouTube audio download manager with integrated tools for metadata editing and
trimming.

This is designed for **downloading live performances and other media not available through your
favourite artists' usual channels**.

(This is not a music piracy tool - do not use it like one. If you _can_ buy, or even just stream,
the music you want to download with this tool, do that instead!)

![A screenshot of CrossPlay's main user interface. It shows a text box for downloading a new song by pasting a YouTube URL, a sort order dropdown, and a settings button. Five songs are listed underneath, with buttons to edit, trim, hide, reset, and delete each one.](img/screenshot.png)

## Features

- Download YouTube videos as an MP3 with `youtube-dl`
- Edit MP3 metadata
- Trim songs, given a start and end point, to remove channel intros or outros
- Hide and show songs
- (Hopefully) cross-platform

## Libraries

CrossPlay's "libraries" are flat folders containing MP3 files, which means you can point your
desktop music player of choice at them, and your songs should be picked up. CrossPlay deliberately
avoids storing any separate databases - if extra information is needed about a song (for example,
where it was downloaded from), this info is stored in a comment within the MP3 file's metadata.

You can also hide songs if you like, which toggles whether they have an MP3 extension, meaning they
should disappear in well-behaved music players.

CrossPlay hasn't been tested too extensively, so regular backups of your library are recommended if
you're planning to use this to archive songs you care about. 

## Dependencies
 
As well as a Rust compiler to actually build this, you'll need the following tools on your PATH:

- `youtube-dl`
- `ffmpeg`
- `gstreamer`
