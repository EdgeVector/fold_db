/**
 * Social media sample data fixtures for the Ingestion tab.
 * Contains Twitter, Instagram, LinkedIn, and TikTok sample posts.
 */

export const twitterSamples = [
  {
    post_id: "tweet_1234567890",
    author: "@techinfluencer",
    author_id: "user_tech_001",
    content:
      "Just launched our new AI-powered database! 🚀 Real-time ingestion, automatic schema mapping, and zero-config setup. Check it out at folddb.io #database #AI #opensource",
    timestamp: "2024-10-21T14:32:00Z",
    likes: 342,
    retweets: 89,
    replies: 23,
    views: 12453,
    media: [
      {
        type: "image",
        url: "https://cdn.example.com/img1.jpg",
        alt: "FoldDB Dashboard Screenshot",
      },
    ],
    mentions: ["@opensource", "@devtools"],
    hashtags: ["database", "AI", "opensource"],
    reply_to: null,
    thread_position: 1,
    engagement_rate: 0.034,
  },
  {
    post_id: "tweet_1234567891",
    author: "@datascientist_pro",
    author_id: "user_ds_042",
    content:
      "Amazing work @techinfluencer! Been testing FoldDB for the past week. The automatic schema inference saved us hours of setup time. Here are my benchmarks:",
    timestamp: "2024-10-21T15:18:00Z",
    likes: 156,
    retweets: 34,
    replies: 12,
    views: 5621,
    media: [
      {
        type: "image",
        url: "https://cdn.example.com/benchmark.png",
        alt: "Performance Benchmarks",
      },
    ],
    mentions: ["@techinfluencer"],
    hashtags: ["database", "performance"],
    reply_to: "tweet_1234567890",
    thread_position: null,
    engagement_rate: 0.036,
  },
];

export const instagramSamples = [
  {
    post_id: "ig_post_987654321",
    username: "foodie_adventures",
    user_id: "ig_user_food_123",
    caption:
      "Best ramen in Tokyo! 🍜✨ The broth was simmering for 48 hours and you can taste every minute of it. Swipe for more pics! #tokyo #ramen #foodie #japan #travel",
    posted_at: "2024-10-20T09:45:00Z",
    location: {
      name: "Ichiran Ramen Shibuya",
      city: "Tokyo",
      country: "Japan",
      coordinates: { lat: 35.6595, lng: 139.7004 },
    },
    media: [
      {
        type: "image",
        url: "https://cdn.instagram.example.com/ramen1.jpg",
        width: 1080,
        height: 1350,
        filter: "Valencia",
      },
      {
        type: "image",
        url: "https://cdn.instagram.example.com/ramen2.jpg",
        width: 1080,
        height: 1350,
        filter: "Valencia",
      },
      {
        type: "image",
        url: "https://cdn.instagram.example.com/ramen3.jpg",
        width: 1080,
        height: 1350,
        filter: "Valencia",
      },
    ],
    likes: 8234,
    comments_count: 456,
    saves: 892,
    shares: 234,
    hashtags: ["tokyo", "ramen", "foodie", "japan", "travel"],
    tagged_users: ["@ramen_tokyo_guide", "@japan_food_official"],
    comments: [
      {
        comment_id: "ig_comment_111",
        username: "tokyo_foodie",
        text: "Omg I was there last week! The tonkotsu broth is incredible 😍",
        timestamp: "2024-10-20T10:12:00Z",
        likes: 45,
      },
      {
        comment_id: "ig_comment_112",
        username: "ramen_lover_88",
        text: "Adding this to my Tokyo bucket list! 📝",
        timestamp: "2024-10-20T11:30:00Z",
        likes: 23,
      },
    ],
  },
  {
    post_id: "ig_post_987654322",
    username: "fitness_journey_2024",
    user_id: "ig_user_fit_456",
    caption:
      "Day 287 of my fitness journey! 💪 Down 45 lbs and feeling stronger than ever. Remember: progress > perfection. What's your fitness goal? #fitness #transformation #motivation #workout",
    posted_at: "2024-10-21T06:00:00Z",
    location: {
      name: "Gold's Gym",
      city: "Los Angeles",
      country: "USA",
      coordinates: { lat: 34.0522, lng: -118.2437 },
    },
    media: [
      {
        type: "video",
        url: "https://cdn.instagram.example.com/workout_vid.mp4",
        thumbnail: "https://cdn.instagram.example.com/workout_thumb.jpg",
        duration: 45,
        width: 1080,
        height: 1920,
      },
    ],
    likes: 15672,
    comments_count: 892,
    saves: 2341,
    shares: 567,
    hashtags: ["fitness", "transformation", "motivation", "workout"],
    tagged_users: ["@personal_trainer_mike"],
    comments: [
      {
        comment_id: "ig_comment_113",
        username: "motivation_daily",
        text: "Incredible transformation! You're an inspiration! 🔥",
        timestamp: "2024-10-21T06:15:00Z",
        likes: 234,
      },
    ],
  },
];

export const linkedinSamples = [
  {
    post_id: "li_post_555666777",
    author: {
      name: "Sarah Chen",
      title: "CTO at TechVentures Inc.",
      profile_url: "linkedin.com/in/sarah-chen-cto",
      user_id: "li_user_sarah_123",
    },
    content:
      "Excited to announce that our team has successfully migrated our entire data infrastructure to a real-time event-driven architecture! 🎉\n\nKey achievements:\n• 10x reduction in data latency (from 5 minutes to 30 seconds)\n• 40% cost savings on infrastructure\n• Improved data quality through automated validation\n• Seamless integration with our ML pipelines\n\nHuge shoutout to the engineering team for their incredible work over the past 6 months. This wouldn't have been possible without their dedication and expertise.\n\nHappy to share more details for anyone interested in event-driven architectures. Feel free to reach out!\n\n#DataEngineering #EventDriven #TechLeadership #Innovation",
    posted_at: "2024-10-21T13:00:00Z",
    article: null,
    media: [
      {
        type: "document",
        title: "Event-Driven Architecture: Our Journey",
        url: "https://cdn.linkedin.example.com/architecture_diagram.pdf",
        pages: 12,
      },
    ],
    reactions: {
      like: 1247,
      celebrate: 342,
      support: 89,
      insightful: 156,
      love: 67,
    },
    comments_count: 87,
    reposts: 234,
    comments: [
      {
        comment_id: "li_comment_aaa111",
        author: {
          name: "Michael Roberts",
          title: "Senior Data Engineer at DataCorp",
          user_id: "li_user_mike_456",
        },
        text: "Congratulations Sarah! We're looking at a similar migration. Would love to connect and learn from your experience.",
        timestamp: "2024-10-21T13:45:00Z",
        reactions: { like: 45 },
      },
      {
        comment_id: "li_comment_aaa112",
        author: {
          name: "Jennifer Liu",
          title: "VP Engineering at CloudScale",
          user_id: "li_user_jen_789",
        },
        text: "Impressive results! The 10x latency improvement is remarkable. Did you use Apache Kafka or another streaming platform?",
        timestamp: "2024-10-21T14:20:00Z",
        reactions: { like: 23, insightful: 8 },
      },
    ],
    industries: ["Technology", "Data Engineering", "Cloud Computing"],
    skills_mentioned: [
      "Event-Driven Architecture",
      "Data Engineering",
      "ML Pipeline",
      "Infrastructure",
    ],
  },
  {
    post_id: "li_post_555666778",
    author: {
      name: "Marcus Thompson",
      title: "Product Manager | Ex-Google | Building the Future of Work",
      profile_url: "linkedin.com/in/marcus-thompson-pm",
      user_id: "li_user_marcus_234",
    },
    content:
      "5 lessons from shipping 100+ product features:\n\n1. Talk to users BEFORE writing specs\n2. Small iterations > big launches\n3. Metrics don't tell the whole story\n4. Technical debt is real debt\n5. Celebrate wins with your team\n\nWhat would you add to this list?\n\n#ProductManagement #Technology #Leadership",
    posted_at: "2024-10-21T10:30:00Z",
    article: null,
    media: [],
    reactions: { like: 3421, celebrate: 892, insightful: 567, love: 234 },
    comments_count: 234,
    reposts: 789,
    comments: [],
    industries: ["Product Management", "Technology", "Startups"],
    skills_mentioned: ["Product Management", "User Research", "Agile"],
  },
];

export const tiktokSamples = [
  {
    video_id: "tt_vid_777888999",
    username: "coding_tips_daily",
    user_id: "tt_user_code_001",
    caption:
      "3 JavaScript array methods that will blow your mind 🤯 #coding #javascript #programming #webdev #learntocode",
    posted_at: "2024-10-21T16:45:00Z",
    video: {
      url: "https://cdn.tiktok.example.com/video_js_tips.mp4",
      thumbnail: "https://cdn.tiktok.example.com/thumb_js_tips.jpg",
      duration: 58,
      width: 1080,
      height: 1920,
      format: "mp4",
    },
    audio: {
      title: "Epic Tech Music",
      artist: "TechBeats Production",
      audio_id: "audio_tech_123",
    },
    statistics: {
      views: 2834562,
      likes: 342891,
      comments: 12453,
      shares: 45672,
      saves: 89234,
      completion_rate: 0.78,
    },
    hashtags: ["coding", "javascript", "programming", "webdev", "learntocode"],
    mentions: [],
    effects: ["Green Screen", "Text Animation", "Transition Effect"],
    comments: [
      {
        comment_id: "tt_comment_xyz1",
        username: "dev_beginner_22",
        text: "Just used .reduce() in my project and it worked perfectly! Thanks!",
        timestamp: "2024-10-21T17:00:00Z",
        likes: 1234,
        replies_count: 45,
      },
      {
        comment_id: "tt_comment_xyz2",
        username: "senior_dev_10yrs",
        text: "Great explanation! Would love to see more advanced array methods",
        timestamp: "2024-10-21T17:30:00Z",
        likes: 892,
        replies_count: 23,
      },
    ],
  },
  {
    video_id: "tt_vid_777889000",
    username: "travel_with_emma",
    user_id: "tt_user_travel_042",
    caption:
      "POV: You visit Santorini for the first time 🇬🇷✨ #travel #santorini #greece #traveltok #wanderlust",
    posted_at: "2024-10-20T08:20:00Z",
    video: {
      url: "https://cdn.tiktok.example.com/video_santorini.mp4",
      thumbnail: "https://cdn.tiktok.example.com/thumb_santorini.jpg",
      duration: 43,
      width: 1080,
      height: 1920,
      format: "mp4",
    },
    audio: {
      title: "Summer Vibes",
      artist: "Chill Beats Co.",
      audio_id: "audio_summer_456",
    },
    statistics: {
      views: 8923451,
      likes: 1234567,
      comments: 34521,
      shares: 123456,
      saves: 234567,
      completion_rate: 0.92,
    },
    hashtags: ["travel", "santorini", "greece", "traveltok", "wanderlust"],
    mentions: ["@visit_greece_official"],
    effects: ["Color Grading", "Slow Motion", "Zoom Transition"],
    location: {
      name: "Santorini",
      country: "Greece",
      coordinates: { lat: 36.3932, lng: 25.4615 },
    },
    comments: [
      {
        comment_id: "tt_comment_xyz3",
        username: "greece_lover_89",
        text: "Adding this to my 2025 bucket list! 😍",
        timestamp: "2024-10-20T09:00:00Z",
        likes: 4521,
        replies_count: 234,
      },
    ],
  },
];
