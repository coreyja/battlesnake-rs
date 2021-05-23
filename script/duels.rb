#!/usr/bin/env ruby

require 'yaml'

BASE_URL = 'http://localhost:8000'.freeze
RUNS = 100

CLI_RESULT_REGEX = /after (.*) turns\. (.*) is the winner/.freeze

class Snake
  attr_reader :name

  def initialize(name)
    @name = name
  end

  def url
    "#{BASE_URL}/#{name}"
  end
end

snake_names = %w[amphibious-arthur devious-devin]
snakes = snake_names.map { |n| Snake.new(n) }

snake_args = snakes.map { |s| "-n #{s.name} -u #{s.url}" }.join ' '

wins = {}

def print_output(wins)
  total_runs = wins.values.flatten.count

  big_winner, winning_turns = wins.max_by { |_, v| v.count }
  winning_pct = winning_turns.count.to_f / total_runs * 100.0

  puts "The big winner is ... #{big_winner}! They won #{winning_pct}% of the rounds (out of #{total_runs} total non draw rounds)"
end

trap("SIGINT") do
  print_output(wins)
  exit!
end

(0...RUNS).each do |i|
  run_result = `battlesnake play #{snake_args} -H 11 -W 11 -t 500 2>&1 >/dev/null | tail -n1`
  match = CLI_RESULT_REGEX.match(run_result)
  if match
    turns, winning_name = match.captures

    puts "#{winning_name} won round #{i} in #{turns} turns"

    wins[winning_name] ||= []
    wins[winning_name] << turns
  else
    puts "Round #{i} was a draw"
  end
end

print_output wins

# puts "FullResults\n#{wins.to_yaml}"
