#!/usr/bin/env bash

curl https://dumps.wikimedia.org/wikidatawiki/20211101/ | grep -Po "wikidatawiki-20211101-pages-meta-history[0-9]+\.xml-[p0-9]+\.7z" | while read -r url ; do
echo $url
wget -c "https://dumps.wikimedia.org/wikidatawiki/20211101/$url"
done