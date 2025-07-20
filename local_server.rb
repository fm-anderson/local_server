#!/usr/bin/env ruby
# frozen_string_literal: true

require 'webrick'
require 'socket'
require 'optparse'
require 'rqrcode_core'

# --- view ---
def render_directory_page(dir_path, request_path, entries)
  html_content = <<~HTML
    <!DOCTYPE html>
    <html lang="en" class="bg-gray-100">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Shared Files</title>
        <script src="https://cdn.tailwindcss.com"></script>
        <link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath stroke='none' d='M0 0h24v24H0z' fill='none'/%3E%3Cpath d='M4 21v-4a3 3 0 0 1 3 -3h5' /%3E%3Cpath d='M9 17l3 -3l-3 -3' /%3E%3Cpath d='M14 3v4a1 1 0 0 0 1 1h4' /%3E%3Cpath d='M5 11v-6a2 2 0 0 1 2 -2h7l5 5v11a2 2 0 0 1 -2 2h-9.5' /%3E%3C/svg%3E">
        <style>body { -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; }</style>
    </head>
    <body class="font-sans text-gray-800">
        <div class="container mx-auto p-4 sm:p-6 lg:p-8">
            <header class="mb-6">
                <h1 class="text-3xl font-bold text-gray-900">Shared Files</h1>
                <p class="text-gray-600 text-sm mt-1 break-words">#{dir_path}</p>
            </header>
            <main class="space-y-4">
  HTML

  entries.each do |name|
    entry_path = File.join(dir_path, name)
    is_folder = File.directory?(entry_path)
    link_href = File.join(request_path, name)

    icon_svg = is_folder ?
      '<svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" /></svg>' :
      '<svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0011.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" /></svg>'

    file_size_info = is_folder ? 'Folder' : format_filesize(File.size(entry_path))

    download_button_html = is_folder ? '' : <<~DOWNLOAD_BUTTON
      <a href="#{link_href}" download class="bg-gray-600 text-white font-bold py-2 px-4 rounded-lg hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-opacity-50 transition-colors">
        Download
      </a>
    DOWNLOAD_BUTTON

    html_content += <<~CARD
      <div class="bg-white rounded-xl shadow-sm border border-gray-200 overflow-hidden">
          <div class="p-4 flex items-center justify-between">
              <div class="flex items-center space-x-4 min-w-0">
                  <div>#{icon_svg}</div>
                  <div class="min-w-0">
                      <p class="font-semibold text-gray-900 truncate">#{name}</p>
                      <p class="text-sm text-gray-500">#{file_size_info}</p>
                  </div>
              </div>
              <a href="#{File.join(request_path, name)}" class="bg-blue-600 text-white font-bold py-2 px-4 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50 transition-colors flex-shrink-0">
                  Open
              </a>
              #{download_button_html}
          </div>
      </div>
    CARD
  end

  html_content += '</main>'
  html_content += <<~FOOTER
    <footer class="text-center mt-8 py-4">
      <p class="text-sm text-gray-500">
        created by
        <a class="text-blue-600 hover:underline" href="https://github.com/fm-anderson" target="_blank" rel="noopener noreferrer">
          fm-anderson
        </a>
      </p>
    </footer>
  FOOTER

  # returns HTML string
  html_content + '</div></body></html>'
end

# --- helpers ---
def get_mime_type(file_path)
  ext = File.extname(file_path).strip.downcase[1..]
  case ext
  when 'html', 'htm' then 'text/html'
  when 'css' then 'text/css'
  when 'js' then 'application/javascript'
  when 'json' then 'application/json'
  when 'jpg', 'jpeg' then 'image/jpeg'
  when 'png' then 'image/png'
  when 'gif' then 'image/gif'
  when 'svg' then 'image/svg+xml'
  when 'pdf' then 'application/pdf'
  when 'zip' then 'application/zip'
  when 'mp4' then 'video/mp4'
  when 'mp3' then 'audio/mpeg'
  else 'application/octet-stream'
  end
end

def format_filesize(bytes)
  return '0 B' if bytes.nil? || bytes.zero?
  units = %w[B KB MB GB TB]
  exp = (Math.log(bytes) / Math.log(1024)).to_i
  exp = 4 if exp > 4
  "%.1f %s" % [bytes.to_f / 1024**exp, units[exp]]
end

def get_local_ip
  addr_info = Socket.ip_address_list.find { |addr| addr.ipv4? && !addr.ipv4_loopback? }
  addr_info&.ip_address
end

# --- config ---
options = { port: 8000, show_qr: false }
OptionParser.new do |opts|
  opts.banner = 'Usage: server.rb [options] [directory]'
  opts.on('-p', '--port PORT', Integer, 'Port to run the server on') do |port|
    options[:port] = port
  end
  opts.on('-qr', '--qrcode', 'Display a QR code for the URL') do
    options[:show_qr] = true
  end
  opts.on('-h', '--help', 'Prints this help') do
    puts opts
    exit
  end
end.parse!

path_arg = ARGV[0] || '.'
unless File.directory?(path_arg)
  puts "Error: '#{path_arg}' is not a valid directory."
  exit 1
end
root = File.expand_path(path_arg)

# --- server ---
server = WEBrick::HTTPServer.new(
  Port: options[:port],
  BindAddress: '0.0.0.0',
  Logger: WEBrick::Log.new(nil, WEBrick::Log::WARN)
)

server.mount_proc '/' do |req, res|
  requested_path = File.join(root, req.path)

  if req.path == '/favicon.ico'
    res.status = 204
    next
  end

  if File.directory?(requested_path)
    entries = Dir.children(requested_path).reject { |f| f.start_with?('.') }.sort
    res.body = render_directory_page(requested_path, req.path, entries)
    res['Content-Type'] = 'text/html'
  elsif File.file?(requested_path)
    res.body = File.read(requested_path)
    res['Content-Type'] = get_mime_type(requested_path)
  else
    res.status = 404
    res.body = "404 Not Found: #{req.path}"
    res['Content-Type'] = 'text/plain'
  end
end

# --- ui ---
url = "http://#{get_local_ip}:#{options[:port]}"

puts '=' * 50
puts 'Server running!'
puts "➡️  URL: #{url}"

if options[:show_qr]
  qr_code = RQRCodeCore::QRCode.new(url, level: :l)
  puts
  puts '➡️  Scan this QR Code with your phone:'
  puts
  puts qr_code.to_s(dark: "\u2588\u2588", light: '  ')
end

puts
puts '=' * 50
puts '(Press Ctrl+C to stop)'

trap('INT') { server.shutdown }
server.start
